#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
account_home="$(getent passwd "$(id -u)" | cut -d: -f6)"
state_root="${XDG_STATE_HOME:-$account_home/.local/state}"
export REVENANT_SECRETS_DIR="${REVENANT_SECRETS_DIR:-$state_root/revenant/m30-secrets/current}"
mode="${1:-normal}"
rendered_source=""
case "$mode" in
  normal)
    compose_files=(-f "$repo_root/infra/docker-compose.yml")
    postgres_ports=0
    ;;
  maintenance)
    compose_files=(-f "$repo_root/infra/docker-compose.yml" -f "$repo_root/infra/docker-compose.maintenance.yml")
    postgres_ports=1
    ;;
  rendered-normal|rendered-maintenance)
    if [[ $# -ne 2 || ! -f "$2" || ! -r "$2" ]]; then
      echo "usage: scripts/m30-validate-compose.sh $mode RENDERED_JSON" >&2
      exit 1
    fi
    rendered_source="$2"
    if [[ "$mode" == rendered-normal ]]; then
      postgres_ports=0
    else
      postgres_ports=1
    fi
    ;;
  *)
    echo "usage: scripts/m30-validate-compose.sh normal | maintenance | rendered-normal RENDERED_JSON | rendered-maintenance RENDERED_JSON" >&2
    exit 1
    ;;
esac

rendered="$(mktemp)"
cleanup() { rm -f "$rendered"; }
trap cleanup EXIT
if [[ -n "$rendered_source" ]]; then
  jq -e 'select(type == "object" and (.services | type == "object"))' \
    "$rendered_source" > "$rendered"
else
  docker compose "${compose_files[@]}" config --format json > "$rendered"
fi

jq -e --argjson postgres_ports "$postgres_ports" '
  (.services.gateway.ports | length) == 2 and
  (.services.inspector.ports | length) == 1 and
  (.services.postgres.ports // [] | length) == $postgres_ports and
  ([.services[] | .ports // [] | .[] | select(.host_ip != "127.0.0.1")] | length) == 0 and
  ((.services.inspector.secrets // []) | length) == 0 and
  ([.services.postgres.secrets[].source] == ["postgres_admin_password"]) and
  ([.services.migrate.secrets[].source] == ["postgres_admin_database_url"]) and
  ([.services["db-provision"].secrets[].source] == ["postgres_runtime_password"]) and
  ([.services.gateway.secrets[].source] == ["gateway_database_url"]) and
  ((.services.gateway.environment // {}) | has("DATABASE_URL") | not) and
  ((.services.postgres.environment // {}) | has("POSTGRES_PASSWORD") | not) and
  (.services.gateway.environment.DATABASE_URL_FILE == "/run/secrets/gateway_database_url") and
  (.services.postgres.environment.POSTGRES_PASSWORD_FILE == "/run/secrets/postgres_admin_password") and
  (.services.gateway.environment.REVENANT_INSPECTOR_ORIGIN ==
    ("http://127.0.0.1:" + (.services.inspector.ports[0].published | tostring))) and
  ([.services[] | .logging == {driver:"local",options:{"max-size":"2m","max-file":"5",compress:"false"}}] | all) and
  ([.services | to_entries[] |
    if .key == "postgres" then ((.value.mem_limit|tonumber) == 268435456 and .value.pids_limit == 64)
    elif .key == "gateway" then ((.value.mem_limit|tonumber) == 134217728 and .value.pids_limit == 192)
    elif .key == "inspector" then ((.value.mem_limit|tonumber) == 67108864 and .value.pids_limit == 32)
    elif (.key == "migrate" or .key == "db-provision") then ((.value.mem_limit|tonumber) == 134217728 and .value.pids_limit == 32)
    else false end] | all)
' "$rendered" >/dev/null || {
  echo "rendered Compose security boundary rejected" >&2
  exit 1
}

printf 'compose_mode=%s\nloopback_publications=pass\npostgres_published_ports=%s\ninspector_secrets=0\ncredential_environment=absent\n' "$mode" "$postgres_ports"
printf 'resource_caps=pass\nlocal_log_rotation=pass\ninspector_origin=exact_loopback\n'
