#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
report_path="${1:-}"
run_token="${M30_MATRIX_RUN_TOKEN:-}"
account_home="$(getent passwd "$(id -u)" | cut -d: -f6)"
state_root="${XDG_STATE_HOME:-$account_home/.local/state}"
secret_root="${REVENANT_SECRETS_ROOT:-$state_root/revenant/m30-secrets}"
current_secrets="${REVENANT_SECRETS_DIR:-$secret_root/current}"
previous_secrets="$secret_root/previous"
base_compose="$repo_root/infra/docker-compose.yml"
maintenance_compose="$repo_root/infra/docker-compose.maintenance.yml"
gateway_image="${M30_GATEWAY_IMAGE:-infra-gateway}"
tls_port="${M30_TLS_PORT:-17443}"
bot_bin="${M30_BOT_BIN:-$repo_root/target/debug/revenant-bot}"
frozen_bin="${M30_FROZEN_BIN:-$repo_root/target/debug/revenant-client-v1}"
persistence_bin="${M30_PERSISTENCE_BIN:-$repo_root/target/debug/persistence-check}"
backup_directory="${REVENANT_VERIFIED_BACKUP_DIR:-}"

if [[ -z "$report_path" || "$report_path" != /* ]] ||
   [[ ! "$run_token" =~ ^[ab]$ ]]; then
  echo "usage: M30_MATRIX_RUN_TOKEN=a|b REVENANT_VERIFIED_BACKUP_DIR=... tests/m30-transport-secrets-matrix.sh ABSOLUTE_REPORT_PATH" >&2
  exit 1
fi
if [[ -e "$report_path" || ! -d "$(dirname "$report_path")" ]]; then
  echo "matrix report target must be a new file in an existing directory" >&2
  exit 1
fi
if [[ "$secret_root" != /* || "$current_secrets" != /* ||
      "$backup_directory" != /* ]]; then
  echo "matrix secret and backup paths must be absolute" >&2
  exit 1
fi
for executable in "$bot_bin" "$frozen_bin" "$persistence_bin"; do
  if [[ ! -x "$executable" ]]; then
    echo "matrix executable is unavailable: $executable" >&2
    exit 1
  fi
done
for command in curl date docker find jq openssl rg sed sha256sum ss stat strings timeout; do
  if ! command -v "$command" >/dev/null; then
    echo "matrix command is unavailable: $command" >&2
    exit 1
  fi
done
if [[ ! -f "$backup_directory/revenant-pre-gate3.pgdump" ]] ||
   ! sha256sum --check --quiet "$backup_directory/revenant-pre-gate3.pgdump.sha256"; then
  echo "verified Gate 3 pre-change backup is unavailable" >&2
  exit 1
fi

export REVENANT_SECRETS_DIR="$current_secrets"
temporary="$(mktemp -d)"
chmod 700 "$temporary"
rows="$temporary/rows.jsonl"
normal_json="$temporary/normal.json"
maintenance_json="$temporary/maintenance.json"
tls_pid=""
maintenance_active=no

cleanup() {
  local status=$?
  trap - EXIT INT TERM
  set +e
  if [[ -n "$tls_pid" ]]; then
    kill "$tls_pid" 2>/dev/null
    wait "$tls_pid" 2>/dev/null
  fi
  if [[ "$maintenance_active" == yes ]]; then
    docker compose -f "$base_compose" up -d postgres >/dev/null 2>&1
  fi
  if [[ -d "$temporary" ]]; then
    find "$temporary" -type f -exec sh -c 'printf "" > "$1"' _ {} \;
    find "$temporary" -type f -delete
    find "$temporary" -depth -mindepth 1 -type d -empty -delete
    rmdir "$temporary" 2>/dev/null
  fi
  exit "$status"
}
trap cleanup EXIT INT TERM

database_digest() {
  docker compose -f "$base_compose" exec -T postgres sh -ec \
    'pg_dump --data-only --inserts --no-owner --no-privileges -U "$POSTGRES_USER" "$POSTGRES_DB"' \
    | sed -E '/^\\(un)?restrict /d' \
    | sha256sum | cut -d' ' -f1
}

millis() {
  date +%s%3N
}

begin_case() {
  case_started="$(millis)"
  case_initial_digest="$(database_digest)"
}

finish_case() {
  local case_id="$1"
  local layer="$2"
  local input_category="$3"
  local expected="$4"
  local observed="$5"
  local mutation_count="$6"
  local state_expectation="${7:-stable}"
  local case_post_digest duration listener_count running_containers passed
  case_post_digest="$(database_digest)"
  duration=$(( $(millis) - case_started ))
  listener_count="$(ss -ltn '( sport = :5432 or sport = :7000 or sport = :8080 or sport = :4173 )' | tail -n +2 | wc -l)"
  running_containers="$(docker compose -f "$base_compose" ps --status running -q | wc -l)"
  passed=true
  if [[ "$expected" != "$observed" ]]; then
    passed=false
  fi
  if [[ "$state_expectation" == stable && "$case_initial_digest" != "$case_post_digest" ]]; then
    passed=false
  fi
  if [[ "$state_expectation" == changed && "$case_initial_digest" == "$case_post_digest" ]]; then
    passed=false
  fi
  jq -cn \
    --arg case_id "$case_id" \
    --arg layer "$layer" \
    --arg input_category "$input_category" \
    --arg initial_state_digest "$case_initial_digest" \
    --arg expected_disposition "$expected" \
    --arg observed_disposition "$observed" \
    --arg post_state_digest "$case_post_digest" \
    --argjson protected_mutation_count "$mutation_count" \
    --argjson duration_ms "$duration" \
    --argjson host_listener_count "$listener_count" \
    --argjson running_containers "$running_containers" \
    --argjson passed "$passed" \
    '{case_id:$case_id,layer:$layer,input_category:$input_category,initial_state_digest:$initial_state_digest,expected_disposition:$expected_disposition,observed_disposition:$observed_disposition,post_state_digest:$post_state_digest,protected_mutation_count:$protected_mutation_count,redaction_result:"pass",duration_ms:$duration_ms,resource_sample:{host_listener_count:$host_listener_count,running_containers:$running_containers},passed:$passed}' \
    >> "$rows"
  if [[ "$passed" != true ]]; then
    echo "$case_id failed its disposition or protected-state boundary" >&2
    return 1
  fi
}

wait_for_health() {
  local service="$1"
  local compose_id health=""
  for _ in {1..120}; do
    compose_id="$(docker compose -f "$base_compose" ps -q "$service")"
    if [[ -n "$compose_id" ]]; then
      health="$(docker inspect "$compose_id" --format '{{.State.Health.Status}}')"
      [[ "$health" == healthy ]] && return
    fi
    sleep 0.25
  done
  echo "$service did not become healthy within the bounded wait" >&2
  return 1
}

probe_password() {
  local role="$1"
  local password_file="$2"
  local password
  password="$(tr -d '\r\n' < "$password_file")"
  if [[ ! "$password" =~ ^[A-Za-z0-9_-]{43}$ ]]; then
    unset password
    return 1
  fi
  {
    printf 'postgres:5432:revenant:%s:%s\n' "$role" "$password"
  } | docker compose -f "$base_compose" run --rm --no-deps -T \
    --entrypoint sh -e PROBE_ROLE="$role" db-provision -ec '
      set -eu
      umask 077
      pass_file=$(mktemp /tmp/revenant-matrix-pgpass-XXXXXX)
      trap '\''printf "" > "$pass_file"; rm -f "$pass_file"'\'' EXIT
      cat > "$pass_file"
      PGPASSFILE="$pass_file" psql -X -h postgres -U "$PROBE_ROLE" -d revenant -Atqc "SELECT 1" >/dev/null
    '
  local status=$?
  unset password
  return "$status"
}

stop_tls() {
  if [[ -n "$tls_pid" ]]; then
    kill "$tls_pid" 2>/dev/null || true
    wait "$tls_pid" 2>/dev/null || true
    tls_pid=""
  fi
}

start_tls() {
  local certificate="$1"
  local key="$2"
  stop_tls
  : > "$temporary/tls-server.log"
  openssl s_server -quiet -www -accept "127.0.0.1:$tls_port" \
    -cert "$certificate" -key "$key" > "$temporary/tls-server.log" 2>&1 &
  tls_pid=$!
  for _ in {1..80}; do
    if ss -ltn "( sport = :$tls_port )" | tail -n +2 | grep -q .; then
      return
    fi
    if ! kill -0 "$tls_pid" 2>/dev/null; then
      echo "isolated TLS endpoint exited before readiness" >&2
      return 1
    fi
    sleep 0.05
  done
  return 1
}

tls_client() {
  local ca_file="$1"
  local hostname="$2"
  printf 'GET / HTTP/1.0\r\nHost: localhost\r\n\r\n' |
    timeout 5 openssl s_client -quiet -verify_return_error \
      -connect "127.0.0.1:$tls_port" -servername localhost \
      -verify_hostname "$hostname" -CAfile "$ca_file" \
      > "$temporary/tls-client.log" 2>&1
}

docker compose -f "$base_compose" config --format json > "$normal_json"
docker compose -f "$base_compose" -f "$maintenance_compose" config --format json \
  > "$maintenance_json"

begin_case
[[ "$(jq '[.services.gateway.ports[].host_ip == "127.0.0.1"] | all' "$normal_json")" == true ]]
[[ "$(ss -ltn '( sport = :7000 or sport = :8080 )' | tail -n +2 | awk '{print $4}' | sort | paste -sd, -)" == "127.0.0.1:7000,127.0.0.1:8080" ]]
finish_case T01 compose gateway_loopback loopback_only loopback_only 0

begin_case
[[ "$(jq '[.services.inspector.ports[].host_ip == "127.0.0.1"] | all' "$normal_json")" == true ]]
[[ "$(ss -ltn '( sport = :4173 )' | tail -n +2 | awk '{print $4}')" == "127.0.0.1:4173" ]]
finish_case T02 compose inspector_loopback loopback_only loopback_only 0

begin_case
[[ "$(jq '(.services.postgres.ports // []) | length' "$normal_json")" == 0 ]]
! ss -ltn '( sport = :5432 )' | tail -n +2 | grep -q .
finish_case T03 compose normal_postgres_publication absent absent 0

begin_case
jq '.services.gateway.ports[0].host_ip = "0.0.0.0"' "$normal_json" > "$temporary/wildcard.json"
jq '.services.gateway.ports[0].host_ip = ""' "$normal_json" > "$temporary/empty.json"
if "$repo_root/scripts/m30-validate-compose.sh" rendered-normal "$temporary/wildcard.json" >/dev/null 2>&1 ||
   "$repo_root/scripts/m30-validate-compose.sh" rendered-normal "$temporary/empty.json" >/dev/null 2>&1; then
  echo "wildcard or empty bind fixture was accepted" >&2
  exit 1
fi
finish_case T04 compose wildcard_or_empty_bind rejected rejected 0

begin_case
for fixture in '192.168.1.20' 'localhost' '::1'; do
  jq --arg host "$fixture" '.services.gateway.ports[0].host_ip = $host' "$normal_json" \
    > "$temporary/nonloopback.json"
  if "$repo_root/scripts/m30-validate-compose.sh" rendered-normal "$temporary/nonloopback.json" >/dev/null 2>&1; then
    echo "non-literal-loopback bind fixture was accepted" >&2
    exit 1
  fi
done
finish_case T05 compose lan_hostname_or_ipv6_bind rejected rejected 0

begin_case
"$repo_root/scripts/m30-validate-compose.sh" rendered-normal "$normal_json" >/dev/null
"$repo_root/scripts/m30-validate-compose.sh" rendered-maintenance "$maintenance_json" >/dev/null
maintenance_active=yes
docker compose -f "$base_compose" -f "$maintenance_compose" up -d postgres >/dev/null
wait_for_health postgres
[[ "$(ss -ltn '( sport = :5432 )' | tail -n +2 | awk '{print $4}')" == "127.0.0.1:5432" ]]
DATABASE_URL_FILE="$current_secrets/operator_database_url" \
REVENANT_EXPECT_ACCOUNT=local:m30-g3-current-v2 "$persistence_bin" >/dev/null
docker compose -f "$base_compose" up -d postgres >/dev/null
maintenance_active=no
wait_for_health postgres
! ss -ltn '( sport = :5432 )' | tail -n +2 | grep -q .
finish_case T06 compose rendered_and_maintenance_audit accepted_then_closed accepted_then_closed 0

tls_directory="$temporary/tls"
mkdir "$tls_directory" "$tls_directory/newcerts"
chmod 700 "$tls_directory"
openssl req -x509 -newkey rsa:2048 -nodes -keyout "$tls_directory/ca.key" \
  -out "$tls_directory/ca.crt" -days 2 -subj /CN=Revenant-M30-CA >/dev/null 2>&1
openssl req -x509 -newkey rsa:2048 -nodes -keyout "$tls_directory/untrusted-ca.key" \
  -out "$tls_directory/untrusted-ca.crt" -days 2 -subj /CN=Untrusted-M30-CA >/dev/null 2>&1
openssl req -newkey rsa:2048 -nodes -keyout "$tls_directory/server.key" \
  -out "$tls_directory/server.csr" -subj /CN=localhost >/dev/null 2>&1
openssl req -newkey rsa:2048 -nodes -keyout "$tls_directory/wrong.key" \
  -out "$tls_directory/wrong.csr" -subj /CN=wrong-key >/dev/null 2>&1
printf 'subjectAltName=DNS:localhost\nextendedKeyUsage=serverAuth\n' > "$tls_directory/server.ext"
openssl x509 -req -in "$tls_directory/server.csr" -CA "$tls_directory/ca.crt" \
  -CAkey "$tls_directory/ca.key" -CAcreateserial -out "$tls_directory/server.crt" \
  -days 2 -extfile "$tls_directory/server.ext" >/dev/null 2>&1
: > "$tls_directory/index.txt"
printf '1000\n' > "$tls_directory/serial"
printf '[ ca ]\ndefault_ca = local_ca\n[ local_ca ]\ndatabase = %s/index.txt\nnew_certs_dir = %s/newcerts\ncertificate = %s/ca.crt\nprivate_key = %s/ca.key\nserial = %s/serial\ndefault_md = sha256\npolicy = policy_any\ncopy_extensions = copy\nunique_subject = no\n[ policy_any ]\ncommonName = supplied\n' \
  "$tls_directory" "$tls_directory" "$tls_directory" "$tls_directory" "$tls_directory" \
  > "$tls_directory/ca.cnf"
start_past="$(date -u -d '3 days ago' +%y%m%d%H%M%SZ)"
end_past="$(date -u -d '2 days ago' +%y%m%d%H%M%SZ)"
start_future="$(date -u -d '2 days' +%y%m%d%H%M%SZ)"
end_future="$(date -u -d '3 days' +%y%m%d%H%M%SZ)"
openssl ca -batch -notext -config "$tls_directory/ca.cnf" -in "$tls_directory/server.csr" \
  -out "$tls_directory/expired.crt" -startdate "$start_past" -enddate "$end_past" \
  -extfile "$tls_directory/server.ext" >/dev/null 2>&1
openssl ca -batch -notext -config "$tls_directory/ca.cnf" -in "$tls_directory/server.csr" \
  -out "$tls_directory/future.crt" -startdate "$start_future" -enddate "$end_future" \
  -extfile "$tls_directory/server.ext" >/dev/null 2>&1
tls_fingerprint="$(openssl x509 -in "$tls_directory/server.crt" -outform DER | sha256sum | cut -d' ' -f1)"

begin_case
start_tls "$tls_directory/server.crt" "$tls_directory/server.key"
if curl --silent --show-error --max-time 2 "http://127.0.0.1:$tls_port/" >/dev/null 2>&1; then
  echo "TLS-required endpoint accepted plaintext" >&2
  exit 1
fi
finish_case T07 isolated_tls plaintext_to_tls_required refused refused 0

begin_case
tls_client "$tls_directory/ca.crt" localhost
finish_case T08 isolated_tls trusted_localhost_certificate accepted accepted 0

begin_case
if tls_client "$tls_directory/untrusted-ca.crt" localhost; then
  echo "untrusted certificate was accepted" >&2
  exit 1
fi
finish_case T09 isolated_tls untrusted_certificate refused refused 0

begin_case
if tls_client "$tls_directory/ca.crt" wrong-host; then
  echo "hostname mismatch was accepted" >&2
  exit 1
fi
finish_case T10 isolated_tls hostname_mismatch refused refused 0

begin_case
start_tls "$tls_directory/expired.crt" "$tls_directory/server.key"
if tls_client "$tls_directory/ca.crt" localhost; then
  echo "expired certificate was accepted" >&2
  exit 1
fi
start_tls "$tls_directory/future.crt" "$tls_directory/server.key"
if tls_client "$tls_directory/ca.crt" localhost; then
  echo "not-yet-valid certificate was accepted" >&2
  exit 1
fi
finish_case T11 isolated_tls invalid_time_window refused refused 0

begin_case
stop_tls
if timeout 3 openssl s_server -quiet -accept "127.0.0.1:$tls_port" \
    -cert "$tls_directory/server.crt" -key "$tls_directory/missing.key" >/dev/null 2>&1 ||
   timeout 3 openssl s_server -quiet -accept "127.0.0.1:$tls_port" \
    -cert "$tls_directory/server.crt" -key "$tls_directory/wrong.key" >/dev/null 2>&1; then
  echo "missing or mismatched TLS key started an endpoint" >&2
  exit 1
fi
finish_case T12 isolated_tls missing_or_mismatched_private_key refused refused 0

begin_case
pattern_file="$temporary/sensitive-patterns"
{
  printf '%s%s\n' '-----BEGIN ' 'PRIVATE KEY-----'
  tr -d '\r\n' < "$current_secrets/postgres_admin_password"; printf '\n'
  tr -d '\r\n' < "$current_secrets/postgres_runtime_password"; printf '\n'
} > "$pattern_file"
docker image save --output "$temporary/gateway-image.tar" "$gateway_image"
if find "$repo_root/client" -type f \( -name '*.key' -o -name '*.pem' -o -name '*.p12' -o -name '*.pfx' \) | grep -q . ||
   rg -a -F -f "$pattern_file" "$temporary/gateway-image.tar" >/dev/null; then
  echo "private or database secret material entered a client or image" >&2
  exit 1
fi
finish_case T13 isolated_tls server_private_material_boundary absent absent 0

begin_case
start_tls "$tls_directory/server.crt" "$tls_directory/server.key"
if tls_client "$tls_directory/untrusted-ca.crt" localhost; then
  echo "untrusted TLS probe unexpectedly succeeded" >&2
  exit 1
fi
tls_client "$tls_directory/ca.crt" localhost
finish_case T14 isolated_tls failed_tls_without_plaintext_retry refused_without_downgrade refused_without_downgrade 0

begin_case
tls_client "$tls_directory/ca.crt" localhost
tls_client "$tls_directory/ca.crt" localhost
stop_tls
finish_case T15 isolated_tls fresh_tls_reconnect accepted accepted 0

begin_case
if rg -n '(:|@)revenant_local(@|/)|DEFAULT_DATABASE_URL|DATABASE_URL:[[:space:]]*postgres|POSTGRES_PASSWORD:[[:space:]]*\$\{[^}]+:-' \
    "$repo_root/infra" "$repo_root/runtime" "$repo_root/tools" "$repo_root/.env.example" >/dev/null; then
  echo "committed default database credential remains" >&2
  exit 1
fi
finish_case T16 secret_configuration committed_default_database_secret absent absent 0

begin_case
jq -e '
  [.services.postgres.secrets[].source] == ["postgres_admin_password"] and
  [.services.migrate.secrets[].source] == ["postgres_admin_database_url"] and
  [.services["db-provision"].secrets[].source] == ["postgres_runtime_password"] and
  [.services.gateway.secrets[].source] == ["gateway_database_url"] and
  ((.services.inspector.secrets // []) | length) == 0
' "$normal_json" >/dev/null
finish_case T17 compose per_service_secret_grants exact exact 0

begin_case
gateway_id="$(docker compose -f "$base_compose" ps -q gateway)"
[[ -n "$gateway_id" ]]
[[ "$(docker inspect "$gateway_id" --format '{{json .Mounts}}' | jq '[.[] | select(.Destination == "/run/secrets/gateway_database_url")] | length')" == 1 ]]
[[ "$(docker inspect "$gateway_id" --format '{{json .Config.Env}}' | jq '[.[] | select(startswith("DATABASE_URL="))] | length')" == 0 ]]
finish_case T18 container gateway_runtime_secret_only exact exact 0

begin_case
inspector_id="$(docker compose -f "$base_compose" ps -q inspector)"
[[ -n "$inspector_id" ]]
[[ "$(docker inspect "$inspector_id" --format '{{json .Mounts}}' | jq 'length')" == 0 ]]
[[ "$(docker inspect "$inspector_id" --format '{{json .Config.Env}}' | jq '[.[] | select(test("(?i)(DATABASE|PASSWORD|SECRET|TOKEN)"))] | length')" == 0 ]]
finish_case T19 container inspector_database_secret absent absent 0

begin_case
marker=m30-redaction-marker-do-not-render
if DATABASE_URL="http://$marker" "$repo_root/target/debug/revenant-gateway" --migrate-only \
    > "$temporary/redaction.log" 2>&1; then
  echo "invalid database configuration was accepted" >&2
  exit 1
fi
if rg -F "$marker" "$temporary/redaction.log" >/dev/null; then
  echo "database diagnostic rendered supplied material" >&2
  exit 1
fi
docker compose -f "$base_compose" logs --no-color > "$temporary/compose.log"
if rg -F -f "$pattern_file" "$temporary/redaction.log" "$temporary/compose.log" >/dev/null; then
  echo "container diagnostics contained secret material" >&2
  exit 1
fi
finish_case T20 diagnostics credential_and_url_redaction redacted redacted 0

begin_case
probe_password revenant "$current_secrets/postgres_admin_password" >/dev/null
probe_password revenant_runtime "$current_secrets/postgres_runtime_password" >/dev/null
role_summary="$(docker compose -f "$base_compose" exec -T postgres sh -ec 'psql -X -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -F "|" <<'\''SQL'\''
SELECT rolname,rolsuper,rolcreaterole,rolcreatedb,rolreplication,rolbypassrls FROM pg_roles WHERE rolname IN ('\''revenant'\'','\''revenant_runtime'\'') ORDER BY rolname;
SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='\''public'\'' AND c.relkind IN ('\''r'\'','\''S'\'') AND pg_get_userbyid(c.relowner)='\''revenant_runtime'\'';
SELECT privilege_type,count(*) FROM information_schema.role_table_grants WHERE grantee='\''revenant_runtime'\'' AND table_schema='\''public'\'' GROUP BY privilege_type ORDER BY privilege_type;
SELECT privilege_type,count(*) FROM information_schema.role_usage_grants WHERE grantee='\''revenant_runtime'\'' AND object_type='\''SEQUENCE'\'' GROUP BY privilege_type ORDER BY privilege_type;
SELECT has_database_privilege('\''revenant_runtime'\'','\''revenant'\'','\''CONNECT'\''),has_database_privilege('\''revenant_runtime'\'','\''revenant'\'','\''CREATE'\''),has_schema_privilege('\''revenant_runtime'\'','\''public'\'','\''USAGE'\''),has_schema_privilege('\''revenant_runtime'\'','\''public'\'','\''CREATE'\'');
SQL')"
expected_roles=$'revenant|t|t|t|t|t\nrevenant_runtime|f|f|f|f|f\n0\nDELETE|1\nINSERT|16\nSELECT|16\nUPDATE|9\nUSAGE|2\nt|f|t|f'
[[ "$role_summary" == "$expected_roles" ]]
finish_case T21 database new_secrets_and_runtime_role accepted_least_privilege accepted_least_privilege 0

begin_case
if probe_password revenant "$previous_secrets/postgres_admin_password" >/dev/null 2>&1 ||
   probe_password revenant_runtime "$previous_secrets/postgres_runtime_password" >/dev/null 2>&1; then
  echo "previous database secret remained valid" >&2
  exit 1
fi
finish_case T22 database previous_database_secrets rejected rejected 0

begin_case
docker compose -f "$base_compose" up -d --no-deps --force-recreate gateway >/dev/null
wait_for_health gateway
probe_password revenant_runtime "$current_secrets/postgres_runtime_password" >/dev/null
finish_case T23 container gateway_restart_with_current_secret healthy healthy 0

begin_case
[[ "$(stat -c '%a' "$secret_root")" == 700 ]]
for directory in "$current_secrets" "$previous_secrets"; do
  [[ "$(stat -c '%a' "$directory")" == 700 ]]
  [[ "$(stat -c '%u' "$directory")" == "$(id -u)" ]]
  [[ "$(find "$directory" -mindepth 1 -maxdepth 1 -type f -perm /077 | wc -l)" == 0 ]]
done
finish_case T24 host_files secret_permissions owner_only owner_only 0

begin_case
scanner_fixture="$repo_root/m30-scanner-fixture-$run_token.key"
if [[ -e "$scanner_fixture" ]]; then
  echo "secret scanner fixture target already exists" >&2
  exit 1
fi
printf 'deliberate scanner fixture\n' > "$scanner_fixture"
if "$repo_root/scripts/audit-secrets.sh" > "$temporary/scanner-reject.log" 2>&1; then
  unlink "$scanner_fixture"
  echo "repository secret scanner accepted a deliberate sensitive path" >&2
  exit 1
fi
unlink "$scanner_fixture"
"$repo_root/scripts/audit-secrets.sh" > "$temporary/scanner-pass.log"
finish_case T25 repository deliberate_secret_scanner_fixture rejected_then_clean rejected_then_clean 0

begin_case
{
  tr -d '\r\n' < "$previous_secrets/postgres_admin_password"; printf '\n'
  tr -d '\r\n' < "$previous_secrets/postgres_runtime_password"; printf '\n'
} >> "$pattern_file"
if rg -a -F -f "$pattern_file" "$temporary/gateway-image.tar" \
    "$backup_directory/revenant-pre-gate3.pgdump" >/dev/null; then
  echo "secret material entered an image or database dump" >&2
  exit 1
fi
grep -qx 'target' "$repo_root/.dockerignore"
grep -qx '.env' "$repo_root/.dockerignore"
finish_case T26 build_context image_and_dump_secret_material absent absent 0

begin_case
[[ "$(sha256sum "$repo_root/archive/clients/v1/src/main.rs" | cut -d' ' -f1)" == "4f481e9fc5d22a5ab6d8f2d0a40e2d05dc9aaf92099debdd9dedf59c26f31f72" ]]
[[ "$(sha256sum "$repo_root/archive/clients/v1/Cargo.toml" | cut -d' ' -f1)" == "c951c5fe88daa2dd9fb91a4da98ca316fd3923e0bff5332d748db44bce367322" ]]
REVENANT_GAME_ADDR=127.0.0.1:7000 "$frozen_bin" > "$temporary/frozen-v1.log"
rg -q 'frozen client V1 build 0.1.0-frozen.*through compatibility adapter' "$temporary/frozen-v1.log"
finish_case T27 compatibility frozen_v1_hashes_and_flow accepted accepted 1 changed

begin_case
REVENANT_GAME_ADDR=127.0.0.1:7000 \
REVENANT_BOT_USERNAME="m30-g3-$run_token-v2" \
  "$bot_bin" > "$temporary/current-v2.log"
rg -q 'handshake accepted by revenant-core using protocol v2' "$temporary/current-v2.log"
rg -q 'activity relay_awakening completed' "$temporary/current-v2.log"
finish_case T28 compatibility current_v2_flow accepted accepted 1 changed

begin_case
REVENANT_GAME_ADDR=127.0.0.1:7000 \
REVENANT_BOT_PROTOCOL_VERSION=3 \
REVENANT_BOT_EXPECT_PROTOCOL_REJECTION=1 \
  "$bot_bin" > "$temporary/unsupported.log"
rg -q 'unsupported protocol v3 refused without negotiation' "$temporary/unsupported.log"
finish_case T29 compatibility unsupported_protocol_generation refused refused 0

begin_case
if rg -i 'protocol[_ -]?v3|protocolversion3|REVENANT_.*TLS|rustls|native.tls' \
    "$repo_root/runtime" "$repo_root/infra" "$repo_root/client" \
    "$repo_root/archive/clients/v1" "$repo_root/tools" \
    --glob '!local-security-lab/**' >/dev/null; then
  echo "ordinary artifact contains a Protocol V3 or default TLS symbol" >&2
  exit 1
fi
if docker compose -f "$base_compose" config | rg -i 'tls|443|protocol.?v3' >/dev/null; then
  echo "normal Compose unexpectedly enables TLS or Protocol V3" >&2
  exit 1
fi
finish_case T30 ordinary_artifacts protocol_v3_or_default_tls absent absent 0

case_count="$(wc -l < "$rows")"
passing_count="$(jq -s '[.[] | select(.passed == true)] | length' "$rows")"
[[ "$case_count" == 30 && "$passing_count" == 30 ]]
[[ "$(jq -sr '[.[].case_id] == [range(1;31) | "T" + (if . < 10 then "0" else "" end) + tostring]' "$rows")" == true ]]

report_temporary="$temporary/report.json"
jq -s \
  --arg schema revenant.m30.transport-secrets-gate.v1 \
  --arg gate M30-Gate-3 \
  --arg tls_server_fingerprint "$tls_fingerprint" \
  --argjson case_count "$case_count" \
  --argjson passing_count "$passing_count" \
  '{schema:$schema,gate:$gate,case_count:$case_count,passing_count:$passing_count,tls_server_public_sha256:$tls_server_fingerprint,cases:.}' \
  "$rows" > "$report_temporary"
umask 077
mv "$report_temporary" "$report_path"
printf 'M30 transport/secrets matrix passed: %s (30/30)\n' "$report_path"
