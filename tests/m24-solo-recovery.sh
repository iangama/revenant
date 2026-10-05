#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$repo_root/scripts/database-url.sh"
gateway_bin="${M24_GATEWAY_BIN:-$repo_root/target/debug/revenant-gateway}"
bot_bin="${M24_BOT_BIN:-$repo_root/target/debug/revenant-bot}"
godot_bin="${GODOT_BIN:-$repo_root/.tooling/godot/Godot_v4.7.1-stable_linux.x86_64}"
health_addr="${M24_RECOVERY_HEALTH_ADDR:-127.0.0.1:18082}"
game_addr="${M24_RECOVERY_GAME_ADDR:-127.0.0.1:17002}"
game_host="${game_addr%:*}"
game_port="${game_addr##*:}"
database_url="$(revenant_database_url)"
evidence_root="${M24_SOLO_EVIDENCE_ROOT:-$repo_root/../revenant-local-evidence}"
package_root="${M24_PACKAGE_ROOT:-$repo_root/../revenant-local-packages}"
package_name="revenant-m24-closed-0.2.0-windows-x86_64-m24-b6-45718926-871f1beafb43"
package_archive="$package_root/$package_name.zip"
package_sha="8cdf81e534b9c2e49472e6cc3cc2c1f4c315114abc22d875767934ed6d62d91e"
run_token="${M24_RECOVERY_RUN_TOKEN:-r$(date -u +%H%M%S)-$$}"

if [[ ! "$run_token" =~ ^[a-zA-Z0-9-]{1,16}$ ]]; then
  echo "M24_RECOVERY_RUN_TOKEN must contain 1-16 ASCII letters, digits, or hyphens" >&2
  exit 1
fi
for executable in "$gateway_bin" "$bot_bin" "$godot_bin"; do
  if [[ ! -x "$executable" ]]; then
    echo "M24 recovery executable is unavailable: $executable" >&2
    exit 1
  fi
done
if [[ "$(sha256sum "$package_archive" | cut -d' ' -f1)" != "$package_sha" ]]; then
  echo "Frozen M24 archive checksum mismatch before recovery" >&2
  exit 1
fi

mkdir -p "$evidence_root"
evidence_dir="$evidence_root/m24-ds-recovery-$run_token"
if [[ -e "$evidence_dir" ]]; then
  echo "M24 recovery evidence target already exists: $evidence_dir" >&2
  exit 1
fi
mkdir "$evidence_dir"

gateway_pid=""
gateway_log=""
client_pid=""
postgres_stopped=0

restore_postgres() {
  if (( postgres_stopped == 1 )); then
    docker compose -f "$repo_root/infra/docker-compose.yml" up -d postgres >/dev/null 2>&1 || true
  fi
}

cleanup() {
  if [[ -n "$client_pid" ]]; then
    kill "$client_pid" 2>/dev/null || true
    wait "$client_pid" 2>/dev/null || true
  fi
  if [[ -n "$gateway_pid" ]]; then
    kill "$gateway_pid" 2>/dev/null || true
    wait "$gateway_pid" 2>/dev/null || true
  fi
  restore_postgres
}
trap cleanup EXIT

psql_value() {
  docker compose -f "$repo_root/infra/docker-compose.yml" exec -T postgres \
    psql -U revenant -d revenant -v ON_ERROR_STOP=1 -Atc "$1"
}

assert_sql() {
  local expected="$1"
  local label="$2"
  local query="$3"
  local actual
  actual="$(psql_value "$query")"
  printf '%s\t%s\t%s\n' "$label" "$expected" "$actual" \
    >>"$evidence_dir/database-assertions.tsv"
  if [[ "$actual" != "$expected" ]]; then
    echo "$label mismatch: expected $expected, got $actual" >&2
    exit 1
  fi
}

wait_for_gateway() {
  for _ in {1..180}; do
    if response="$(curl --fail --silent "http://$health_addr/health" 2>/dev/null)"; then
      [[ "$response" == *'"status":"ok"'* ]]
      return 0
    fi
    sleep 0.1
  done
  return 1
}

wait_for_postgres() {
  for _ in {1..120}; do
    if docker compose -f "$repo_root/infra/docker-compose.yml" exec -T postgres \
      pg_isready -U revenant -d revenant >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.25
  done
  return 1
}

wait_for_log() {
  local log="$1"
  local pattern="$2"
  for _ in {1..240}; do
    if grep -q "$pattern" "$log" 2>/dev/null; then
      return 0
    fi
    sleep 0.05
  done
  return 1
}

wait_for_resets() {
  local expected="$1"
  for _ in {1..240}; do
    if [[ "$(grep -c '"event":"session_reset"' "$gateway_log" || true)" -ge "$expected" ]]; then
      return 0
    fi
    sleep 0.05
  done
  return 1
}

start_gateway() {
  local label="$1"
  gateway_log="$evidence_dir/gateway-$label.log"
  REVENANT_BIND_ADDR="$health_addr" \
  REVENANT_GAME_ADDR="$game_addr" \
  REVENANT_EXPECTED_PLAYERS=1 \
  REVENANT_DATABASE_MODE=existing \
  DATABASE_URL="$database_url" \
    "$gateway_bin" >"$gateway_log" 2>&1 &
  gateway_pid=$!
  if ! wait_for_gateway; then
    cat "$gateway_log" >&2
    echo "M24 recovery gateway did not become healthy for $label" >&2
    exit 1
  fi
}

stop_gateway() {
  kill "$gateway_pid"
  wait "$gateway_pid" 2>/dev/null || true
  gateway_pid=""
}

run_bot() {
  local username="$1"
  local role="$2"
  local output="$3"
  REVENANT_GAME_ADDR="$game_addr" \
  REVENANT_BOT_USERNAME="$username" \
  REVENANT_BOT_ROLE="$role" \
  REVENANT_EXPECTED_PLAYERS=1 \
    timeout 30s "$bot_bin" >"$output" 2>&1
}

run_probe() {
  local mode="$1"
  local username="$2"
  local expected="$3"
  local output="$4"
  M24_PROBE_MODE="$mode" \
  M24_PROBE_HOST="$game_host" \
  M24_PROBE_PORT="$game_port" \
  M24_PROBE_USERNAME="$username" \
  M24_PROBE_EXPECTED_OUTCOME="$expected" \
  XDG_DATA_HOME="$evidence_dir/probe-data/data" \
  XDG_CONFIG_HOME="$evidence_dir/probe-data/config" \
  XDG_CACHE_HOME="$evidence_dir/probe-data/cache" \
    timeout 15s "$godot_bin" --headless --path "$evidence_dir/probe-project" \
      --script res://m24-solo-protocol-probe.gd >"$output" 2>&1
}

printf 'assertion\texpected\tactual\n' >"$evidence_dir/database-assertions.tsv"
started_epoch="$(date +%s)"

mkdir "$evidence_dir/probe-project" "$evidence_dir/probe-data"
(
  cd "$repo_root/client/game"
  tar --exclude='./.godot' -cf - .
) | (
  cd "$evidence_dir/probe-project"
  tar -xf -
)
cp "$repo_root/tests/fixtures/m24-solo-protocol-probe.gd" \
  "$evidence_dir/probe-project/m24-solo-protocol-probe.gd"
XDG_DATA_HOME="$evidence_dir/probe-data/data" \
XDG_CONFIG_HOME="$evidence_dir/probe-data/config" \
XDG_CACHE_HOME="$evidence_dir/probe-data/cache" \
  "$godot_bin" --headless --editor --path "$evidence_dir/probe-project" --quit \
    >"$evidence_dir/probe-import.log" 2>&1

refusal_started="$(date +%s%3N)"
run_probe admission "m24ds-$run_token-refused" transport_failure \
  "$evidence_dir/refusal.log"
refusal_elapsed=$(( $(date +%s%3N) - refusal_started ))
grep -q 'outcome=transport_failure' "$evidence_dir/refusal.log"

start_gateway primary
run_probe handshake "m24ds-$run_token-handshake" rejected \
  "$evidence_dir/handshake.log"
grep -q 'outcome=rejected protocol=99' "$evidence_dir/handshake.log"

occupied_username="m24ds-$run_token-occ"
busy_username="m24ds-$run_token-busy"
REVENANT_GAME_ADDR="$game_addr" \
REVENANT_BOT_USERNAME="$occupied_username" \
REVENANT_BOT_ROLE=observer \
REVENANT_EXPECTED_PLAYERS=1 \
  "$bot_bin" >"$evidence_dir/occupied-holder.log" 2>&1 &
client_pid=$!
wait_for_log "$evidence_dir/occupied-holder.log" 'activity relay_awakening started'
run_probe admission "$busy_username" session_unavailable "$evidence_dir/occupied-probe.log"
grep -q 'outcome=session_unavailable' "$evidence_dir/occupied-probe.log"
kill -KILL "$client_pid"
wait "$client_pid" 2>/dev/null || true
client_pid=""
wait_for_resets 1

occupied_account="local:$occupied_username"
busy_account="local:$busy_username"
assert_sql 1 occupied-incomplete-session \
  "SELECT count(DISTINCT session_id) FROM replay_events WHERE account_id = '$occupied_account' AND event_type = 'player_joined';"
assert_sql 0 occupied-incomplete-reward \
  "SELECT count(*) FROM activity_history WHERE account_id = '$occupied_account';"
assert_sql 0 occupied-rejected-join \
  "SELECT count(*) FROM replay_events WHERE account_id = '$busy_account';"

run_bot "$occupied_username" driver "$evidence_dir/occupied-retry.log"
wait_for_resets 2
assert_sql 2 occupied-distinct-sessions \
  "SELECT count(DISTINCT session_id) FROM replay_events WHERE account_id = '$occupied_account' AND event_type = 'player_joined';"
assert_sql 1 occupied-retry-history \
  "SELECT count(*) FROM activity_history WHERE account_id = '$occupied_account';"
assert_sql 1 occupied-retry-fragment \
  "SELECT i.quantity FROM inventory i JOIN characters c ON c.id = i.character_id WHERE c.account_id = '$occupied_account' AND i.item_id = 'relay_core_fragment';"
assert_sql 100 occupied-retry-experience \
  "SELECT p.experience FROM progression p JOIN characters c ON c.id = p.character_id WHERE c.account_id = '$occupied_account';"

restart_username="m24ds-$run_token-restart"
restart_account="local:$restart_username"
run_bot "$restart_username" driver "$evidence_dir/restart-before.log"
wait_for_resets 3
stop_gateway
start_gateway restarted
run_bot "$restart_username" driver "$evidence_dir/restart-after.log"
wait_for_resets 1
assert_sql 2 restart-history \
  "SELECT count(*) FROM activity_history WHERE account_id = '$restart_account';"
assert_sql 2 restart-fragments \
  "SELECT i.quantity FROM inventory i JOIN characters c ON c.id = i.character_id WHERE c.account_id = '$restart_account' AND i.item_id = 'relay_core_fragment';"
assert_sql 200 restart-experience \
  "SELECT p.experience FROM progression p JOIN characters c ON c.id = p.character_id WHERE c.account_id = '$restart_account';"

postgres_container_before="$(docker inspect -f '{{.Id}}' infra-postgres-1)"
gateway_pid_before_outage="$gateway_pid"
postgres_username="m24ds-$run_token-pg"
postgres_account="local:$postgres_username"
REVENANT_GAME_ADDR="$game_addr" \
REVENANT_BOT_USERNAME="$postgres_username" \
REVENANT_BOT_ROLE=driver \
REVENANT_EXPECTED_PLAYERS=1 \
  "$bot_bin" >"$evidence_dir/postgres-interrupted.log" 2>&1 &
client_pid=$!
wait_for_log "$evidence_dir/postgres-interrupted.log" 'activity relay_awakening started'
docker compose -f "$repo_root/infra/docker-compose.yml" stop postgres \
  >"$evidence_dir/postgres-stop.log" 2>&1
postgres_stopped=1
if wait "$client_pid"; then
  echo "M24 PostgreSQL interruption client unexpectedly completed" >&2
  client_pid=""
  exit 1
fi
client_pid=""
wait_for_log "$gateway_log" 'session_aborted_after_command_failure'
docker compose -f "$repo_root/infra/docker-compose.yml" up -d postgres \
  >"$evidence_dir/postgres-start.log" 2>&1
postgres_stopped=0
wait_for_postgres
postgres_container_after="$(docker inspect -f '{{.Id}}' infra-postgres-1)"
if [[ "$postgres_container_after" != "$postgres_container_before" ]]; then
  echo "M24 PostgreSQL recovery replaced the existing container" >&2
  exit 1
fi
if [[ "$gateway_pid" != "$gateway_pid_before_outage" ]]; then
  echo "M24 PostgreSQL recovery restarted the local gateway" >&2
  exit 1
fi

run_bot "$postgres_username" driver "$evidence_dir/postgres-retry.log"
wait_for_log "$gateway_log" 'session_persistence_reconnected'
assert_sql 2 postgres-distinct-sessions \
  "SELECT count(DISTINCT session_id) FROM replay_events WHERE account_id = '$postgres_account' AND event_type = 'player_joined';"
assert_sql 1 postgres-completed-session \
  "SELECT count(DISTINCT session_id) FROM replay_events WHERE account_id = '$postgres_account' AND event_type = 'activity_completed';"
assert_sql 1 postgres-history \
  "SELECT count(*) FROM activity_history WHERE account_id = '$postgres_account';"
assert_sql 1 postgres-fragment \
  "SELECT i.quantity FROM inventory i JOIN characters c ON c.id = i.character_id WHERE c.account_id = '$postgres_account' AND i.item_id = 'relay_core_fragment';"
assert_sql 100 postgres-experience \
  "SELECT p.experience FROM progression p JOIN characters c ON c.id = p.character_id WHERE c.account_id = '$postgres_account';"
assert_sql 1 postgres-inventory-grant \
  "SELECT count(*) FROM inventory_reward_grants g JOIN characters c ON c.id = g.character_id WHERE c.account_id = '$postgres_account';"
assert_sql 1 postgres-progression-grant \
  "SELECT count(*) FROM progression_reward_grants g JOIN characters c ON c.id = g.character_id WHERE c.account_id = '$postgres_account';"
assert_sql 1 postgres-loot-replay \
  "SELECT count(*) FROM replay_events WHERE account_id = '$postgres_account' AND event_type = 'loot_granted';"
assert_sql 1 postgres-progression-replay \
  "SELECT count(*) FROM replay_events WHERE account_id = '$postgres_account' AND event_type = 'progression_granted';"
assert_sql 0 temporary-failure-triggers \
  "SELECT count(*) FROM pg_trigger WHERE NOT tgisinternal AND tgname LIKE 'm24_fail_%';"
assert_sql 0 temporary-failure-functions \
  "SELECT count(*) FROM pg_proc WHERE proname LIKE 'm24_fail_%';"

stop_gateway

if [[ "$(sha256sum "$package_archive" | cut -d' ' -f1)" != "$package_sha" ]]; then
  echo "Frozen M24 archive checksum changed during recovery" >&2
  exit 1
fi

finished_epoch="$(date +%s)"
printf 'run_token=%s\nrefusal_elapsed_ms=%s\noccupied_outcome=session_unavailable\nhandshake_outcome=rejected\npostgres_container_unchanged=true\ngateway_pid_during_outage=%s\nduration_seconds=%s\npackage_sha256=%s\n' \
  "$run_token" "$refusal_elapsed" "$gateway_pid_before_outage" \
  "$((finished_epoch - started_epoch))" "$package_sha" \
  >"$evidence_dir/summary.txt"

echo "M24 solo recovery matrix passed"
echo "M24_SOLO_RECOVERY_EVIDENCE=$evidence_dir"
