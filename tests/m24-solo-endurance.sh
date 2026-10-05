#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$repo_root/scripts/database-url.sh"
gateway_bin="${M24_GATEWAY_BIN:-$repo_root/target/debug/revenant-gateway}"
bot_bin="${M24_BOT_BIN:-$repo_root/target/debug/revenant-bot}"
solo_runs="${M24_SOLO_RUNS:-20}"
multi_runs="${M24_MULTI_RUNS:-10}"
health_addr="${M24_HEALTH_ADDR:-127.0.0.1:18081}"
game_addr="${M24_GAME_ADDR:-127.0.0.1:17001}"
database_url="$(revenant_database_url)"
evidence_root="${M24_SOLO_EVIDENCE_ROOT:-$repo_root/../revenant-local-evidence}"
package_root="${M24_PACKAGE_ROOT:-$repo_root/../revenant-local-packages}"
package_name="revenant-m24-closed-0.2.0-windows-x86_64-m24-b6-45718926-871f1beafb43"
package_archive="$package_root/$package_name.zip"
package_sha="8cdf81e534b9c2e49472e6cc3cc2c1f4c315114abc22d875767934ed6d62d91e"
run_token="${M24_SOLO_RUN_TOKEN:-$(date -u +%H%M%S)-$$}"

if [[ ! "$solo_runs" =~ ^[0-9]+$ || "$solo_runs" -lt 2 ]]; then
  echo "M24_SOLO_RUNS must be an integer of at least 2" >&2
  exit 1
fi
if [[ ! "$multi_runs" =~ ^[0-9]+$ || "$multi_runs" -lt 1 ]]; then
  echo "M24_MULTI_RUNS must be a positive integer" >&2
  exit 1
fi
if [[ ! "$run_token" =~ ^[a-zA-Z0-9-]{1,16}$ ]]; then
  echo "M24_SOLO_RUN_TOKEN must contain 1-16 ASCII letters, digits, or hyphens" >&2
  exit 1
fi
if [[ ! -x "$gateway_bin" || ! -x "$bot_bin" ]]; then
  echo "M24 endurance requires compiled gateway and bot binaries" >&2
  exit 1
fi
if [[ ! -f "$package_archive" ]]; then
  echo "Frozen M24 archive is unavailable: $package_archive" >&2
  exit 1
fi
if [[ "$(sha256sum "$package_archive" | cut -d' ' -f1)" != "$package_sha" ]]; then
  echo "Frozen M24 archive checksum mismatch before endurance" >&2
  exit 1
fi

mkdir -p "$evidence_root"
evidence_dir="$evidence_root/m24-ds-$run_token"
if [[ -e "$evidence_dir" ]]; then
  echo "M24 endurance evidence target already exists: $evidence_dir" >&2
  exit 1
fi
mkdir "$evidence_dir"

gateway_pid=""
observer_pid=""
gateway_log=""

cleanup() {
  if [[ -n "$observer_pid" ]]; then
    kill "$observer_pid" 2>/dev/null || true
    wait "$observer_pid" 2>/dev/null || true
  fi
  if [[ -n "$gateway_pid" ]]; then
    kill "$gateway_pid" 2>/dev/null || true
    wait "$gateway_pid" 2>/dev/null || true
  fi
}
trap cleanup EXIT

psql_value() {
  docker compose -f "$repo_root/infra/docker-compose.yml" exec -T postgres \
    psql -U revenant -d revenant -v ON_ERROR_STOP=1 -Atc "$1"
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

wait_for_resets() {
  local expected="$1"
  for _ in {1..200}; do
    if [[ "$(grep -c '"event":"session_reset"' "$gateway_log" || true)" -ge "$expected" ]]; then
      return 0
    fi
    sleep 0.05
  done
  return 1
}

start_gateway() {
  local expected_players="$1"
  local label="$2"
  gateway_log="$evidence_dir/gateway-$label.log"
  REVENANT_BIND_ADDR="$health_addr" \
  REVENANT_GAME_ADDR="$game_addr" \
  REVENANT_EXPECTED_PLAYERS="$expected_players" \
  REVENANT_DATABASE_MODE=existing \
  DATABASE_URL="$database_url" \
    "$gateway_bin" >"$gateway_log" 2>&1 &
  gateway_pid=$!
  if ! wait_for_gateway; then
    cat "$gateway_log" >&2
    echo "M24 endurance gateway did not become healthy for $label" >&2
    exit 1
  fi
}

stop_gateway() {
  kill "$gateway_pid"
  wait "$gateway_pid" 2>/dev/null || true
  gateway_pid=""
}

capture_metrics() {
  local phase="$1"
  local rss_kib
  local descriptors
  local threads
  local connections
  rss_kib="$(ps -o rss= -p "$gateway_pid" | tr -d ' ')"
  descriptors="$(find "/proc/$gateway_pid/fd" -mindepth 1 -maxdepth 1 | wc -l)"
  threads="$(sed -n 's/^Threads:[[:space:]]*//p' "/proc/$gateway_pid/status")"
  connections="$(psql_value "SELECT count(*) FROM pg_stat_activity WHERE datname = 'revenant' AND usename = 'revenant';")"
  printf '%s\t%s\t%s\t%s\t%s\n' \
    "$phase" "$rss_kib" "$descriptors" "$threads" "$connections" \
    >>"$evidence_dir/resource-metrics.tsv"
  METRIC_RSS="$rss_kib"
  METRIC_FDS="$descriptors"
  METRIC_THREADS="$threads"
  METRIC_CONNECTIONS="$connections"
}

assert_resource_bounds() {
  local label="$1"
  local before_rss="$2"
  local before_fds="$3"
  local before_threads="$4"
  local before_connections="$5"
  local rss_delta=$((METRIC_RSS - before_rss))
  if (( rss_delta > 65536 )); then
    echo "$label gateway RSS grew by more than 64 MiB: ${rss_delta} KiB" >&2
    exit 1
  fi
  if (( METRIC_FDS > before_fds + 4 )); then
    echo "$label gateway retained too many descriptors: $before_fds -> $METRIC_FDS" >&2
    exit 1
  fi
  if (( METRIC_THREADS > before_threads + 4 )); then
    echo "$label gateway retained too many threads: $before_threads -> $METRIC_THREADS" >&2
    exit 1
  fi
  if (( METRIC_CONNECTIONS > before_connections + 2 )); then
    echo "$label retained too many PostgreSQL connections: $before_connections -> $METRIC_CONNECTIONS" >&2
    exit 1
  fi
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

run_bot() {
  local username="$1"
  local role="$2"
  local expected_players="$3"
  local output="$4"
  REVENANT_GAME_ADDR="$game_addr" \
  REVENANT_BOT_USERNAME="$username" \
  REVENANT_BOT_ROLE="$role" \
  REVENANT_EXPECTED_PLAYERS="$expected_players" \
    timeout 30s "$bot_bin" >"$output" 2>&1
}

printf 'phase\trss_kib\tdescriptors\tthreads\tpostgres_connections\n' \
  >"$evidence_dir/resource-metrics.tsv"
printf 'assertion\texpected\tactual\n' >"$evidence_dir/database-assertions.tsv"
started_epoch="$(date +%s)"
fresh_runs=$((solo_runs / 2))
reuse_runs=$((solo_runs - fresh_runs))
solo_prefix="local:m24ds-$run_token-s"
multi_prefix="local:m24ds-$run_token-m"
reuse_username="m24ds-$run_token-sr"
reuse_account="local:$reuse_username"

start_gateway 1 solo
capture_metrics solo-before
solo_before_rss="$METRIC_RSS"
solo_before_fds="$METRIC_FDS"
solo_before_threads="$METRIC_THREADS"
solo_before_connections="$METRIC_CONNECTIONS"

for run in $(seq 1 "$solo_runs"); do
  if (( run <= fresh_runs )); then
    username="m24ds-$run_token-sf$(printf '%02d' "$run")"
  else
    username="$reuse_username"
  fi
  run_bot "$username" driver 1 "$evidence_dir/solo-$(printf '%02d' "$run").log"
  wait_for_resets "$run"
  printf 'M24 solo endurance %d/%d completed\n' "$run" "$solo_runs"
done

sleep 0.2
capture_metrics solo-after
assert_resource_bounds solo "$solo_before_rss" "$solo_before_fds" \
  "$solo_before_threads" "$solo_before_connections"
stop_gateway

expected_solo_accounts=$((fresh_runs + 1))
assert_sql "$expected_solo_accounts" solo-account-count \
  "SELECT count(*) FROM accounts WHERE id LIKE '$solo_prefix%';"
assert_sql "$solo_runs" solo-history-total \
  "SELECT count(*) FROM activity_history WHERE account_id LIKE '$solo_prefix%' AND activity_id = 'relay_awakening';"
assert_sql "$solo_runs" solo-fragment-total \
  "SELECT COALESCE(sum(i.quantity), 0) FROM inventory i JOIN characters c ON c.id = i.character_id WHERE c.account_id LIKE '$solo_prefix%' AND i.item_id = 'relay_core_fragment';"
assert_sql "$((solo_runs * 100))" solo-experience-total \
  "SELECT COALESCE(sum(p.experience), 0) FROM progression p JOIN characters c ON c.id = p.character_id WHERE c.account_id LIKE '$solo_prefix%';"
assert_sql "$solo_runs" solo-inventory-grants \
  "SELECT count(*) FROM inventory_reward_grants g JOIN characters c ON c.id = g.character_id WHERE c.account_id LIKE '$solo_prefix%';"
assert_sql "$solo_runs" solo-progression-grants \
  "SELECT count(*) FROM progression_reward_grants g JOIN characters c ON c.id = g.character_id WHERE c.account_id LIKE '$solo_prefix%';"
assert_sql "$solo_runs" solo-loot-replay \
  "SELECT count(*) FROM replay_events WHERE account_id LIKE '$solo_prefix%' AND event_type = 'loot_granted';"
assert_sql "$solo_runs" solo-progression-replay \
  "SELECT count(*) FROM replay_events WHERE account_id LIKE '$solo_prefix%' AND event_type = 'progression_granted';"
assert_sql "$reuse_runs" solo-reused-history \
  "SELECT count(*) FROM activity_history WHERE account_id = '$reuse_account' AND activity_id = 'relay_awakening';"
assert_sql "$reuse_runs" solo-reused-fragments \
  "SELECT COALESCE(i.quantity, 0) FROM characters c LEFT JOIN inventory i ON i.character_id = c.id AND i.item_id = 'relay_core_fragment' WHERE c.account_id = '$reuse_account';"
assert_sql "$((reuse_runs * 100))" solo-reused-experience \
  "SELECT p.experience FROM progression p JOIN characters c ON c.id = p.character_id WHERE c.account_id = '$reuse_account';"
assert_sql 0 solo-account-integrity \
  "WITH selected AS (SELECT id FROM accounts WHERE id LIKE '$solo_prefix%') SELECT count(*) FROM selected a WHERE (SELECT count(*) FROM activity_history h WHERE h.account_id = a.id AND h.activity_id = 'relay_awakening') <> CASE WHEN a.id = '$reuse_account' THEN $reuse_runs ELSE 1 END OR (SELECT COALESCE(i.quantity, 0) FROM characters c LEFT JOIN inventory i ON i.character_id = c.id AND i.item_id = 'relay_core_fragment' WHERE c.account_id = a.id) <> CASE WHEN a.id = '$reuse_account' THEN $reuse_runs ELSE 1 END OR (SELECT p.experience FROM progression p JOIN characters c ON c.id = p.character_id WHERE c.account_id = a.id) <> CASE WHEN a.id = '$reuse_account' THEN $((reuse_runs * 100)) ELSE 100 END;"
assert_sql "$solo_runs" solo-session-count \
  "SELECT count(DISTINCT session_id) FROM replay_events WHERE account_id LIKE '$solo_prefix%' AND event_type = 'player_joined';"
assert_sql 0 solo-session-integrity \
  "WITH selected AS (SELECT DISTINCT session_id FROM replay_events WHERE account_id LIKE '$solo_prefix%' AND event_type = 'player_joined'), counts AS (SELECT r.session_id, count(*) FILTER (WHERE event_type = 'player_joined') joins, count(*) FILTER (WHERE event_type = 'activity_completed') completions, count(*) FILTER (WHERE event_type = 'loot_granted') loot, count(*) FILTER (WHERE event_type = 'progression_granted') progression, count(*) FILTER (WHERE event_type = 'equipment_changed') equipment, count(*) FILTER (WHERE event_type = 'enemy_died') deaths, count(*) FILTER (WHERE event_type = 'boss_spawned') bosses FROM replay_events r JOIN selected s ON s.session_id = r.session_id GROUP BY r.session_id) SELECT count(*) FROM counts WHERE joins <> 1 OR completions <> 1 OR loot <> 1 OR progression <> 1 OR equipment <> 1 OR deaths <> 2 OR bosses <> 1;"

start_gateway 2 multiplayer
capture_metrics multiplayer-before
multi_before_rss="$METRIC_RSS"
multi_before_fds="$METRIC_FDS"
multi_before_threads="$METRIC_THREADS"
multi_before_connections="$METRIC_CONNECTIONS"

for run in $(seq 1 "$multi_runs"); do
  observer="m24ds-$run_token-mo$(printf '%02d' "$run")"
  driver="m24ds-$run_token-md$(printf '%02d' "$run")"
  run_bot "$observer" observer 2 "$evidence_dir/multi-$(printf '%02d' "$run")-observer.log" &
  observer_pid=$!
  sleep 0.15
  run_bot "$driver" driver 2 "$evidence_dir/multi-$(printf '%02d' "$run")-driver.log"
  wait "$observer_pid"
  observer_pid=""
  wait_for_resets "$run"
  printf 'M24 multiplayer endurance %d/%d completed\n' "$run" "$multi_runs"
done

sleep 0.2
capture_metrics multiplayer-after
assert_resource_bounds multiplayer "$multi_before_rss" "$multi_before_fds" \
  "$multi_before_threads" "$multi_before_connections"
stop_gateway

expected_multi_accounts=$((multi_runs * 2))
assert_sql "$expected_multi_accounts" multiplayer-account-count \
  "SELECT count(*) FROM accounts WHERE id LIKE '$multi_prefix%';"
assert_sql "$expected_multi_accounts" multiplayer-history-total \
  "SELECT count(*) FROM activity_history WHERE account_id LIKE '$multi_prefix%' AND activity_id = 'relay_awakening';"
assert_sql "$expected_multi_accounts" multiplayer-fragment-total \
  "SELECT COALESCE(sum(i.quantity), 0) FROM inventory i JOIN characters c ON c.id = i.character_id WHERE c.account_id LIKE '$multi_prefix%' AND i.item_id = 'relay_core_fragment';"
assert_sql "$((expected_multi_accounts * 100))" multiplayer-experience-total \
  "SELECT COALESCE(sum(p.experience), 0) FROM progression p JOIN characters c ON c.id = p.character_id WHERE c.account_id LIKE '$multi_prefix%';"
assert_sql "$expected_multi_accounts" multiplayer-inventory-grants \
  "SELECT count(*) FROM inventory_reward_grants g JOIN characters c ON c.id = g.character_id WHERE c.account_id LIKE '$multi_prefix%';"
assert_sql "$expected_multi_accounts" multiplayer-progression-grants \
  "SELECT count(*) FROM progression_reward_grants g JOIN characters c ON c.id = g.character_id WHERE c.account_id LIKE '$multi_prefix%';"
assert_sql "$expected_multi_accounts" multiplayer-loot-replay \
  "SELECT count(*) FROM replay_events WHERE account_id LIKE '$multi_prefix%' AND event_type = 'loot_granted';"
assert_sql "$expected_multi_accounts" multiplayer-progression-replay \
  "SELECT count(*) FROM replay_events WHERE account_id LIKE '$multi_prefix%' AND event_type = 'progression_granted';"
assert_sql 0 multiplayer-account-integrity \
  "WITH selected AS (SELECT id FROM accounts WHERE id LIKE '$multi_prefix%') SELECT count(*) FROM selected a WHERE (SELECT count(*) FROM activity_history h WHERE h.account_id = a.id AND h.activity_id = 'relay_awakening') <> 1 OR (SELECT COALESCE(i.quantity, 0) FROM characters c LEFT JOIN inventory i ON i.character_id = c.id AND i.item_id = 'relay_core_fragment' WHERE c.account_id = a.id) <> 1 OR (SELECT p.experience FROM progression p JOIN characters c ON c.id = p.character_id WHERE c.account_id = a.id) <> 100;"
assert_sql "$multi_runs" multiplayer-session-count \
  "SELECT count(DISTINCT session_id) FROM replay_events WHERE account_id LIKE '$multi_prefix%' AND event_type = 'player_joined';"
assert_sql 0 multiplayer-session-integrity \
  "WITH selected AS (SELECT DISTINCT session_id FROM replay_events WHERE account_id LIKE '$multi_prefix%' AND event_type = 'player_joined'), counts AS (SELECT r.session_id, count(*) FILTER (WHERE event_type = 'player_joined') joins, count(*) FILTER (WHERE event_type = 'activity_completed') completions, count(*) FILTER (WHERE event_type = 'loot_granted') loot, count(*) FILTER (WHERE event_type = 'progression_granted') progression, count(*) FILTER (WHERE event_type = 'equipment_changed') equipment, count(*) FILTER (WHERE event_type = 'enemy_died') deaths, count(*) FILTER (WHERE event_type = 'boss_spawned') bosses FROM replay_events r JOIN selected s ON s.session_id = r.session_id GROUP BY r.session_id) SELECT count(*) FROM counts WHERE joins <> 2 OR completions <> 1 OR loot <> 2 OR progression <> 2 OR equipment <> 1 OR deaths <> 2 OR bosses <> 1;"

if [[ "$(sha256sum "$package_archive" | cut -d' ' -f1)" != "$package_sha" ]]; then
  echo "Frozen M24 archive checksum changed during endurance" >&2
  exit 1
fi

finished_epoch="$(date +%s)"
duration_seconds=$((finished_epoch - started_epoch))
printf 'run_token=%s\nsolo_runs=%s\nsolo_fresh_accounts=%s\nsolo_reused_runs=%s\nmulti_sessions=%s\nmulti_accounts=%s\nduration_seconds=%s\npackage_sha256=%s\n' \
  "$run_token" "$solo_runs" "$fresh_runs" "$reuse_runs" "$multi_runs" \
  "$expected_multi_accounts" "$duration_seconds" "$package_sha" \
  >"$evidence_dir/summary.txt"

echo "M24 solo endurance passed: $solo_runs solo and $multi_runs two-client sessions"
echo "M24_SOLO_EVIDENCE=$evidence_dir"
