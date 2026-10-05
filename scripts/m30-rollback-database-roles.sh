#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
account_home="$(getent passwd "$(id -u)" | cut -d: -f6)"
state_root="${XDG_STATE_HOME:-$account_home/.local/state}"
secret_root="${REVENANT_SECRETS_ROOT:-$state_root/revenant/m30-secrets}"
current_directory="$secret_root/current"
previous_directory="$secret_root/previous"
compose_file="${REVENANT_COMPOSE_FILE:-$repo_root/infra/docker-compose.yml}"
backup_directory="${REVENANT_VERIFIED_BACKUP_DIR:-}"

if [[ "$secret_root" != /* || "$backup_directory" != /* ]]; then
  echo "secret root and verified backup directory must be absolute paths" >&2
  exit 1
fi
checksum_file="$backup_directory/revenant-pre-gate3.pgdump.sha256"
if [[ ! -f "$checksum_file" ]] || ! sha256sum --check --quiet "$checksum_file"; then
  echo "verified pre-change database backup is unavailable or invalid" >&2
  exit 1
fi

validate_generation() {
  local directory="$1"
  if [[ ! -d "$directory" || "$(stat -c '%a' "$directory")" != 700 ]] ||
     [[ "$(stat -c '%u' "$directory")" != "$(id -u)" ]] ||
     [[ "$(find "$directory" -mindepth 1 -maxdepth 1 -type f | wc -l)" -ne 5 ]]; then
    echo "complete owner-only secret generation is unavailable for rollback" >&2
    return 1
  fi
  local name
  for name in postgres_admin_password postgres_runtime_password \
    postgres_admin_database_url gateway_database_url operator_database_url; do
    if [[ ! -f "$directory/$name" || "$(stat -c '%a' "$directory/$name")" != 600 ]] ||
       [[ "$(stat -c '%u' "$directory/$name")" != "$(id -u)" ]]; then
      echo "rollback secret generation file is unavailable or not owner-only" >&2
      return 1
    fi
  done
}

validate_generation "$current_directory"
validate_generation "$previous_directory"
if [[ -e "$secret_root/.rotation-lock" ]]; then
  echo "another database secret operation is already active" >&2
  exit 1
fi
install -d -m 700 "$secret_root/.rotation-lock"
export REVENANT_SECRETS_DIR="$current_directory"

database_digest() {
  docker compose -f "$compose_file" exec -T postgres sh -ec \
    'pg_dump --data-only --inserts --no-owner --no-privileges -U "$POSTGRES_USER" "$POSTGRES_DB"' \
    | sed -E '/^\\(un)?restrict /d' \
    | sha256sum | cut -d' ' -f1
}

apply_passwords() {
  local admin_password="$1"
  local runtime_password="$2"
  {
    printf 'BEGIN;\n'
    printf "ALTER ROLE revenant PASSWORD '%s';\n" "$admin_password"
    printf "ALTER ROLE revenant_runtime PASSWORD '%s';\n" "$runtime_password"
    printf 'COMMIT;\n'
  } | docker compose -f "$compose_file" exec -T postgres sh -ec '
    set -eu
    umask 077
    sql_file=$(mktemp /tmp/revenant-rollback-XXXXXX)
    trap '\''printf "" > "$sql_file"; rm -f "$sql_file"'\'' EXIT
    cat > "$sql_file"
    psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -f "$sql_file" >/dev/null
  '
}

probe_password() {
  local role="$1"
  local password="$2"
  {
    printf 'postgres:5432:revenant:%s:%s\n' "$role" "$password"
  } | docker compose -f "$compose_file" run --rm --no-deps -T \
    --entrypoint sh -e PROBE_ROLE="$role" db-provision -ec '
    set -eu
    umask 077
    pass_file=$(mktemp /tmp/revenant-rollback-pgpass-XXXXXX)
    trap '\''printf "" > "$pass_file"; rm -f "$pass_file"'\'' EXIT
    cat > "$pass_file"
    PGPASSFILE="$pass_file" psql -X -h postgres -U "$PROBE_ROLE" -d revenant -Atqc "SELECT 1" >/dev/null
  '
}

run_job() {
  local service="$1"
  docker compose -f "$compose_file" up -d --no-deps --force-recreate "$service" >/dev/null
  local state="" exit_code=""
  for _ in {1..120}; do
    state="$(docker inspect "$(docker compose -f "$compose_file" ps -a -q "$service")" --format '{{.State.Status}}')"
    if [[ "$state" == exited ]]; then
      exit_code="$(docker inspect "$(docker compose -f "$compose_file" ps -a -q "$service")" --format '{{.State.ExitCode}}')"
      [[ "$exit_code" == 0 ]]
      return
    fi
    sleep 0.25
  done
  return 1
}

restart_dependants() {
  export REVENANT_SECRETS_DIR="$current_directory"
  run_job migrate
  run_job db-provision
  docker compose -f "$compose_file" up -d --no-deps --force-recreate gateway >/dev/null
  local health=""
  for _ in {1..120}; do
    health="$(docker inspect "$(docker compose -f "$compose_file" ps -q gateway)" --format '{{.State.Health.Status}}')"
    [[ "$health" == healthy ]] && return
    sleep 0.25
  done
  return 1
}

current_admin_password="$(tr -d '\r\n' < "$current_directory/postgres_admin_password")"
current_runtime_password="$(tr -d '\r\n' < "$current_directory/postgres_runtime_password")"
previous_admin_password="$(tr -d '\r\n' < "$previous_directory/postgres_admin_password")"
previous_runtime_password="$(tr -d '\r\n' < "$previous_directory/postgres_runtime_password")"
for password in "$current_admin_password" "$current_runtime_password" \
  "$previous_admin_password" "$previous_runtime_password"; do
  if [[ ! "$password" =~ ^[A-Za-z0-9_-]{43}$ ]]; then
    echo "rollback secret has an invalid shape" >&2
    rmdir "$secret_root/.rotation-lock"
    exit 1
  fi
done
pre_digest="$(database_digest)"
swapped=no

restore_failed_rollback() {
  local status=$?
  trap - ERR EXIT
  set +e
  apply_passwords "$current_admin_password" "$current_runtime_password" >/dev/null 2>&1
  if [[ "$swapped" == yes ]]; then
    swap_directory="$secret_root/.rollback-swap-$$"
    mv "$current_directory" "$swap_directory"
    mv "$previous_directory" "$current_directory"
    mv "$swap_directory" "$previous_directory"
  fi
  restart_dependants >/dev/null 2>&1
  rmdir "$secret_root/.rotation-lock" 2>/dev/null
  unset current_admin_password current_runtime_password previous_admin_password previous_runtime_password
  echo "database secret rollback failed and the current generation was restored" >&2
  exit "$status"
}
trap restore_failed_rollback ERR EXIT

apply_passwords "$previous_admin_password" "$previous_runtime_password"
probe_password revenant "$previous_admin_password"
probe_password revenant_runtime "$previous_runtime_password"
if probe_password revenant "$current_admin_password" >/dev/null 2>&1 ||
   probe_password revenant_runtime "$current_runtime_password" >/dev/null 2>&1; then
  echo "replaced database secret remained valid during rollback" >&2
  false
fi

swap_directory="$secret_root/.rollback-swap-$$"
mv "$current_directory" "$swap_directory"
mv "$previous_directory" "$current_directory"
mv "$swap_directory" "$previous_directory"
swapped=yes
restart_dependants

post_digest="$(database_digest)"
if [[ "$pre_digest" != "$post_digest" ]]; then
  echo "working database data changed during secret rollback" >&2
  false
fi
probe_password revenant "$previous_admin_password"
probe_password revenant_runtime "$previous_runtime_password"
if probe_password revenant "$current_admin_password" >/dev/null 2>&1 ||
   probe_password revenant_runtime "$current_runtime_password" >/dev/null 2>&1; then
  echo "replaced database secret worked after rollback restart" >&2
  false
fi

trap - ERR EXIT
rmdir "$secret_root/.rotation-lock"
unset current_admin_password current_runtime_password previous_admin_password previous_runtime_password
printf '{"event":"database_secret_rollback","result":"pass","restored_connections":"pass","replaced_connections":"rejected","dependants":"healthy","data_digest":"preserved"}\n'
