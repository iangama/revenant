#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
account_home="$(getent passwd "$(id -u)" | cut -d: -f6)"
state_root="${XDG_STATE_HOME:-$account_home/.local/state}"
secret_root="${REVENANT_SECRETS_ROOT:-$state_root/revenant/m30-secrets}"
current_directory="$secret_root/current"
previous_directory="$secret_root/previous"
candidate_name="${1:-}"
backup_directory="${REVENANT_VERIFIED_BACKUP_DIR:-}"
compose_file="${REVENANT_COMPOSE_FILE:-$repo_root/infra/docker-compose.yml}"

if [[ ! "$candidate_name" =~ ^[a-z0-9][a-z0-9-]{0,31}$ ]] ||
   [[ "$candidate_name" == current || "$candidate_name" == previous ]]; then
  echo "usage: REVENANT_VERIFIED_BACKUP_DIR=... scripts/m30-rotate-database-secrets.sh CANDIDATE_GENERATION" >&2
  exit 1
fi
if [[ "$secret_root" != /* || "$backup_directory" != /* ]]; then
  echo "secret root and verified backup directory must be absolute paths" >&2
  exit 1
fi
candidate_directory="$secret_root/$candidate_name"

for command in docker find sha256sum stat; do
  if ! command -v "$command" >/dev/null; then
    echo "required rotation command is unavailable: $command" >&2
    exit 1
  fi
done

checksum_file="$backup_directory/revenant-pre-gate3.pgdump.sha256"
if [[ ! -f "$checksum_file" ]] || ! sha256sum --check --quiet "$checksum_file"; then
  echo "verified pre-change database backup is unavailable or invalid" >&2
  exit 1
fi

validate_generation() {
  local directory="$1"
  if [[ ! -d "$directory" || "$(stat -c '%a' "$directory")" != 700 ]] ||
     [[ "$(stat -c '%u' "$directory")" != "$(id -u)" ]]; then
    echo "secret generation directory is not owner-controlled" >&2
    return 1
  fi
  local expected=(
    gateway_database_url
    operator_database_url
    postgres_admin_database_url
    postgres_admin_password
    postgres_runtime_password
  )
  if [[ "$(find "$directory" -mindepth 1 -maxdepth 1 -type f | wc -l)" -ne 5 ]] ||
     [[ "$(find "$directory" -mindepth 1 -maxdepth 1 ! -type f | wc -l)" -ne 0 ]]; then
    echo "secret generation does not contain exactly five regular files" >&2
    return 1
  fi
  local name path
  for name in "${expected[@]}"; do
    path="$directory/$name"
    if [[ ! -f "$path" || ! -r "$path" || "$(stat -c '%a' "$path")" != 600 ]] ||
       [[ "$(stat -c '%u' "$path")" != "$(id -u)" ]]; then
      echo "secret generation file is unavailable or not owner-only: $name" >&2
      return 1
    fi
  done
  local admin_password runtime_password
  admin_password="$(tr -d '\r\n' < "$directory/postgres_admin_password")"
  runtime_password="$(tr -d '\r\n' < "$directory/postgres_runtime_password")"
  if [[ ! "$admin_password" =~ ^[A-Za-z0-9_-]{43}$ ]] ||
     [[ ! "$runtime_password" =~ ^[A-Za-z0-9_-]{43}$ ]] ||
     [[ "$admin_password" == "$runtime_password" ]] ||
     [[ "$(tr -d '\r\n' < "$directory/postgres_admin_database_url")" != "postgres://revenant:$admin_password@postgres:5432/revenant" ]] ||
     [[ "$(tr -d '\r\n' < "$directory/gateway_database_url")" != "postgres://revenant_runtime:$runtime_password@postgres:5432/revenant" ]] ||
     [[ "$(tr -d '\r\n' < "$directory/operator_database_url")" != "postgres://revenant_runtime:$runtime_password@127.0.0.1:5432/revenant" ]]; then
    unset admin_password runtime_password
    echo "secret generation contents have an invalid or inconsistent shape" >&2
    return 1
  fi
  unset admin_password runtime_password
}

validate_generation "$current_directory"
validate_generation "$candidate_directory"

if [[ -e "$secret_root/.rotation-lock" ]]; then
  echo "another database secret rotation is already active" >&2
  exit 1
fi
install -d -m 700 "$secret_root/.rotation-lock"

export REVENANT_SECRETS_DIR="$current_directory"
if ss -ltn '( sport = :5432 )' | tail -n +2 | grep -q .; then
  rmdir "$secret_root/.rotation-lock"
  echo "normal PostgreSQL must not have a host listener during rotation" >&2
  exit 1
fi

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
    sql_file=$(mktemp /tmp/revenant-rotate-XXXXXX)
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
    pass_file=$(mktemp /tmp/revenant-rotate-pgpass-XXXXXX)
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
  echo "$service did not complete within the bounded rotation wait" >&2
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
  echo "Gateway did not become healthy after secret rotation" >&2
  return 1
}

secure_remove_previous() {
  if [[ ! -e "$previous_directory" ]]; then
    return
  fi
  if [[ "$previous_directory" != "$secret_root/previous" ]] ||
     [[ ! -d "$previous_directory" ]] ||
     [[ "$(stat -c '%u' "$previous_directory")" != "$(id -u)" ]]; then
    echo "refusing to replace an unexpected previous generation" >&2
    return 1
  fi
  find "$previous_directory" -mindepth 1 -maxdepth 1 -type f \
    -exec sh -c 'printf "" > "$1"' _ {} \;
  find "$previous_directory" -mindepth 1 -maxdepth 1 -type f -delete
  rmdir "$previous_directory"
}

old_admin_password="$(tr -d '\r\n' < "$current_directory/postgres_admin_password")"
old_runtime_password="$(tr -d '\r\n' < "$current_directory/postgres_runtime_password")"
new_admin_password="$(tr -d '\r\n' < "$candidate_directory/postgres_admin_password")"
new_runtime_password="$(tr -d '\r\n' < "$candidate_directory/postgres_runtime_password")"
pre_digest="$(database_digest)"
swapped=no

rollback_failed_rotation() {
  local status=$?
  trap - ERR EXIT
  set +e
  apply_passwords "$old_admin_password" "$old_runtime_password" >/dev/null 2>&1
  if [[ "$swapped" == yes ]]; then
    failed_directory="$secret_root/.failed-$candidate_name-$$"
    mv "$current_directory" "$failed_directory"
    mv "$previous_directory" "$current_directory"
    find "$failed_directory" -mindepth 1 -maxdepth 1 -type f \
      -exec sh -c 'printf "" > "$1"' _ {} \;
    find "$failed_directory" -mindepth 1 -maxdepth 1 -type f -delete
    rmdir "$failed_directory"
  fi
  restart_dependants >/dev/null 2>&1
  rmdir "$secret_root/.rotation-lock" 2>/dev/null
  unset old_admin_password old_runtime_password new_admin_password new_runtime_password
  echo "database secret rotation failed and the previous generation was restored" >&2
  exit "$status"
}
trap rollback_failed_rotation ERR EXIT

apply_passwords "$new_admin_password" "$new_runtime_password"
probe_password revenant "$new_admin_password"
probe_password revenant_runtime "$new_runtime_password"
if probe_password revenant "$old_admin_password" >/dev/null 2>&1 ||
   probe_password revenant_runtime "$old_runtime_password" >/dev/null 2>&1; then
  echo "an old database secret remained valid after rotation" >&2
  false
fi

secure_remove_previous
mv "$current_directory" "$previous_directory"
mv "$candidate_directory" "$current_directory"
swapped=yes
restart_dependants

post_digest="$(database_digest)"
if [[ "$pre_digest" != "$post_digest" ]]; then
  echo "working database data changed during secret rotation" >&2
  false
fi
probe_password revenant "$new_admin_password"
probe_password revenant_runtime "$new_runtime_password"
if probe_password revenant "$old_admin_password" >/dev/null 2>&1 ||
   probe_password revenant_runtime "$old_runtime_password" >/dev/null 2>&1; then
  echo "an old database secret worked after dependent restart" >&2
  false
fi

trap - ERR EXIT
rmdir "$secret_root/.rotation-lock"
unset old_admin_password old_runtime_password new_admin_password new_runtime_password
printf '{"event":"database_secret_rotation","result":"pass","new_connections":"pass","old_connections":"rejected","dependants":"healthy","data_digest":"preserved","previous_generation":"protected"}\n'
