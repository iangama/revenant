#!/usr/bin/env bash
set -euo pipefail

mode="${1:---container}"
database_name="${POSTGRES_DB:-revenant}"
admin_role="${POSTGRES_USER:-revenant}"
runtime_role="${POSTGRES_RUNTIME_USER:-revenant_runtime}"

for value in "$database_name" "$admin_role" "$runtime_role"; do
  if [[ ! "$value" =~ ^[a-z][a-z0-9_]{0,31}$ ]]; then
    echo "database and role names must be bounded lowercase SQL identifiers" >&2
    exit 1
  fi
done

validate_password() {
  [[ "$1" =~ ^[A-Za-z0-9_-]{43}$ ]]
}

render_sql() {
  local admin_password="$1"
  local runtime_password="$2"
  local admin_password_statement=""
  if [[ -n "$admin_password" ]]; then
    admin_password_statement="ALTER ROLE \"$admin_role\" WITH LOGIN SUPERUSER CREATEDB CREATEROLE INHERIT REPLICATION BYPASSRLS PASSWORD '$admin_password';"
  fi
  cat <<SQL
BEGIN;
$admin_password_statement
DO \$provision\$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = '$runtime_role') THEN
    CREATE ROLE "$runtime_role";
  END IF;
END
\$provision\$;
ALTER ROLE "$runtime_role" WITH LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE INHERIT NOREPLICATION NOBYPASSRLS PASSWORD '$runtime_password';
REVOKE ALL PRIVILEGES ON DATABASE "$database_name" FROM "$runtime_role";
REVOKE CREATE ON DATABASE "$database_name" FROM PUBLIC;
GRANT CONNECT ON DATABASE "$database_name" TO "$runtime_role";
REVOKE ALL PRIVILEGES ON SCHEMA public FROM "$runtime_role";
REVOKE CREATE ON SCHEMA public FROM PUBLIC;
GRANT USAGE ON SCHEMA public TO "$runtime_role";
REVOKE ALL PRIVILEGES ON ALL TABLES IN SCHEMA public FROM "$runtime_role";
REVOKE ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public FROM "$runtime_role";
GRANT SELECT ON ALL TABLES IN SCHEMA public TO "$runtime_role";
GRANT INSERT ON TABLE
  accounts, characters, inventory, equipment_loadouts, activity_history,
  inventory_reward_grants, progression, progression_reward_grants,
  module_states, module_loadout_slots, module_operations, route_operations,
  route_operation_participants, replay_events, cooperation_operations,
  cooperation_operation_participants, acquisition_milestones, acquisition_claims
TO "$runtime_role";
GRANT UPDATE ON TABLE
  accounts, characters, inventory, equipment_loadouts, progression,
  module_states, route_operations, cooperation_operations,
  cooperation_operation_participants
TO "$runtime_role";
GRANT DELETE ON TABLE module_loadout_slots TO "$runtime_role";
GRANT USAGE ON SEQUENCE activity_history_id_seq, replay_events_id_seq TO "$runtime_role";
ALTER DEFAULT PRIVILEGES FOR ROLE "$admin_role" IN SCHEMA public REVOKE ALL ON TABLES FROM "$runtime_role";
ALTER DEFAULT PRIVILEGES FOR ROLE "$admin_role" IN SCHEMA public REVOKE ALL ON SEQUENCES FROM "$runtime_role";
COMMIT;
SQL
}

capture_previous_verifiers() {
  local output_directory="$1"
  if [[ -e "$output_directory" ]]; then
    echo "refusing to overwrite previous role verifiers" >&2
    return 1
  fi
  install -d -m 700 "$output_directory"
  docker compose -f "$compose_file" exec -T postgres sh -ec \
    'psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -F "|" -c "SELECT rolname, rolpassword FROM pg_authid WHERE rolname IN ('\''revenant'\'', '\''revenant_runtime'\'') ORDER BY rolname"' \
    > "$output_directory/role-verifiers.txt"
  chmod 600 "$output_directory/role-verifiers.txt"
  if ! rg -q '^revenant\|SCRAM-SHA-256\$' "$output_directory/role-verifiers.txt"; then
    echo "administrator rollback verifier was not captured" >&2
    return 1
  fi
}

if [[ "$mode" == "--container" ]]; then
  runtime_password="$(tr -d '\r\n' < /run/secrets/postgres_runtime_password)"
  if ! validate_password "$runtime_password"; then
    echo "mounted database secret has an invalid shape" >&2
    exit 1
  fi
  umask 077
  sql_file="$(mktemp /tmp/revenant-provision-XXXXXX)"
  pass_file="$(mktemp /tmp/revenant-pgpass-XXXXXX)"
  cleanup() {
    printf '' > "$sql_file" 2>/dev/null || true
    printf '' > "$pass_file" 2>/dev/null || true
    rm -f "$sql_file" "$pass_file"
  }
  trap cleanup EXIT
  render_sql "" "$runtime_password" > "$sql_file"
  psql -X -v ON_ERROR_STOP=1 -h "${POSTGRES_ADMIN_HOST:-/var/run/postgresql}" \
    -U "$admin_role" -d "$database_name" -f "$sql_file" >/dev/null
  printf '%s:5432:%s:%s:%s\n' "${POSTGRES_HOST:-postgres}" "$database_name" "$runtime_role" "$runtime_password" > "$pass_file"
  unset runtime_password
  PGPASSFILE="$pass_file" psql -X -v ON_ERROR_STOP=1 -h "${POSTGRES_HOST:-postgres}" -U "$runtime_role" -d "$database_name" -Atqc 'SELECT 1' >/dev/null
  echo '{"event":"database_runtime_provisioned","result":"pass"}'
  exit 0
fi

if [[ "$mode" != "--bootstrap-existing" ]]; then
  echo "usage: scripts/m30-provision-database.sh --container | --bootstrap-existing" >&2
  exit 1
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
compose_file="${REVENANT_COMPOSE_FILE:-$repo_root/infra/docker-compose.yml}"
account_home="$(getent passwd "$(id -u)" | cut -d: -f6)"
state_root="${XDG_STATE_HOME:-$account_home/.local/state}"
secret_directory="${REVENANT_SECRETS_DIR:-$state_root/revenant/m30-secrets/current}"
previous_directory="${REVENANT_PREVIOUS_SECRETS_DIR:-$state_root/revenant/m30-secrets/previous}"
export REVENANT_SECRETS_DIR="$secret_directory"
admin_password="$(tr -d '\r\n' < "$secret_directory/postgres_admin_password")"
runtime_password="$(tr -d '\r\n' < "$secret_directory/postgres_runtime_password")"
if ! validate_password "$admin_password" || ! validate_password "$runtime_password"; then
  echo "host database secret has an invalid shape" >&2
  exit 1
fi
capture_previous_verifiers "$previous_directory"
render_sql "$admin_password" "$runtime_password" |
  docker compose -f "$compose_file" exec -T postgres sh -ec '
    set -eu
    umask 077
    sql_file=$(mktemp /tmp/revenant-provision-XXXXXX)
    trap '\''printf "" > "$sql_file"; rm -f "$sql_file"'\'' EXIT
    cat > "$sql_file"
    psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -f "$sql_file" >/dev/null
  '
umask 077
probe_file="$(mktemp)"
cleanup_probe() { printf '' > "$probe_file" 2>/dev/null || true; rm -f "$probe_file"; }
trap cleanup_probe EXIT
printf '127.0.0.1:5432:%s:%s:%s\n' "$database_name" "$admin_role" "$admin_password" > "$probe_file"
unset admin_password
docker compose -f "$compose_file" exec -T -e PGPASSFILE=/tmp/revenant-bootstrap-pgpass postgres sh -ec 'umask 077; cat > "$PGPASSFILE"; trap '\''printf "" > "$PGPASSFILE"; rm -f "$PGPASSFILE"'\'' EXIT; psql -X -h 127.0.0.1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -Atqc "SELECT 1" >/dev/null' < "$probe_file"
printf '127.0.0.1:5432:%s:%s:%s\n' "$database_name" "$runtime_role" "$runtime_password" > "$probe_file"
unset runtime_password
docker compose -f "$compose_file" exec -T -e PGPASSFILE=/tmp/revenant-bootstrap-pgpass postgres sh -ec 'umask 077; cat > "$PGPASSFILE"; trap '\''printf "" > "$PGPASSFILE"; rm -f "$PGPASSFILE"'\'' EXIT; psql -X -h 127.0.0.1 -U revenant_runtime -d "$POSTGRES_DB" -Atqc "SELECT 1" >/dev/null' < "$probe_file"
echo '{"event":"database_runtime_bootstrapped","new_connections":"pass","previous_verifier":"protected"}'
