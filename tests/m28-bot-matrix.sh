#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$repo_root/scripts/database-url.sh"
gateway_bin="${M28_GATEWAY_BIN:-$repo_root/target/debug/revenant-gateway}"
bot_bin="${M28_COOPERATION_BOT_BIN:-$repo_root/target/debug/revenant-cooperation-bot}"
health_addr="${M28_BOT_HEALTH_ADDR:-127.0.0.1:18086}"
game_addr="${M28_BOT_GAME_ADDR:-127.0.0.1:17006}"
inspector_origin="${REVENANT_INSPECTOR_ORIGIN:-http://127.0.0.1:4173}"
database_url="$(revenant_database_url)"
evidence_root="${M28_BOT_EVIDENCE_ROOT:-$repo_root/../revenant-local-evidence}"
run_token="${M28_BOT_RUN_TOKEN:-$(date -u +%m%d%H%M)-$$}"

if [[ ! "$run_token" =~ ^[a-zA-Z0-9-]{1,10}$ ]]; then
  echo "M28_BOT_RUN_TOKEN must contain 1-10 ASCII letters, digits, or hyphens" >&2
  exit 1
fi
for executable in "$gateway_bin" "$bot_bin"; do
  if [[ ! -x "$executable" ]]; then
    echo "M28 bot matrix executable is unavailable: $executable" >&2
    exit 1
  fi
done
for command in awk curl docker find jq rg sha256sum timeout; do
  if ! command -v "$command" >/dev/null; then
    echo "M28 bot matrix command is unavailable: $command" >&2
    exit 1
  fi
done

mkdir -p "$evidence_root"
evidence_dir="$evidence_root/m28-block6-bots-$run_token"
if [[ -e "$evidence_dir" ]]; then
  echo "M28 bot matrix evidence target already exists: $evidence_dir" >&2
  exit 1
fi
mkdir "$evidence_dir"
gateway_log="$evidence_dir/gateway.log"
gateway_pid=""

cleanup() {
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

sample_resources() {
  local sample="$1"
  local rss_kib
  local descriptors
  local threads
  local connections
  rss_kib="$(awk '/^VmRSS:/ {print $2}' "/proc/$gateway_pid/status")"
  descriptors="$(find "/proc/$gateway_pid/fd" -mindepth 1 -maxdepth 1 -type l | wc -l)"
  threads="$(find "/proc/$gateway_pid/task" -mindepth 1 -maxdepth 1 -type d | wc -l)"
  connections="$(psql_value "SELECT count(*) FROM pg_stat_activity WHERE datname='revenant' AND usename='revenant';")"
  printf '%s\t%s\t%s\t%s\t%s\n' "$sample" "$rss_kib" "$descriptors" "$threads" "$connections" \
    >>"$evidence_dir/resources.tsv"
  if [[ "$sample" == before ]]; then
    RESOURCE_RSS_BEFORE="$rss_kib"
    RESOURCE_FDS_BEFORE="$descriptors"
    RESOURCE_THREADS_BEFORE="$threads"
    RESOURCE_CONNECTIONS_BEFORE="$connections"
  else
    RESOURCE_RSS_AFTER="$rss_kib"
    RESOURCE_FDS_AFTER="$descriptors"
    RESOURCE_THREADS_AFTER="$threads"
    RESOURCE_CONNECTIONS_AFTER="$connections"
  fi
}

assert_resource_growth() {
  local label="$1"
  local before="$2"
  local after="$3"
  local maximum="$4"
  local growth=$((after - before))
  if ((growth < 0)); then
    growth=0
  fi
  printf '%s\t%s\t%s\n' "$label" "$growth" "$maximum" >>"$evidence_dir/resource-assertions.tsv"
  if ((growth > maximum)); then
    echo "$label resource growth exceeded: $growth > $maximum" >&2
    exit 1
  fi
}

assert_sql() {
  local expected="$1"
  local label="$2"
  local query="$3"
  local actual
  actual="$(psql_value "$query")"
  printf '%s\t%s\t%s\n' "$label" "$expected" "$actual" >>"$evidence_dir/database-assertions.tsv"
  if [[ "$actual" != "$expected" ]]; then
    echo "$label mismatch: expected $expected, got $actual" >&2
    exit 1
  fi
}

REVENANT_BIND_ADDR="$health_addr" \
REVENANT_GAME_ADDR="$game_addr" \
REVENANT_EXPECTED_PLAYERS=2 \
REVENANT_DATABASE_MODE=existing \
DATABASE_URL="$database_url" \
  "$gateway_bin" >"$gateway_log" 2>&1 &
gateway_pid=$!
for _ in {1..180}; do
  if curl --fail --silent "http://$health_addr/health" >/dev/null; then
    break
  fi
  if ! kill -0 "$gateway_pid" 2>/dev/null; then
    sed -n '1,240p' "$gateway_log" >&2
    echo "M28 bot matrix gateway exited before readiness" >&2
    exit 1
  fi
  sleep 0.1
done
curl --fail --silent "http://$health_addr/health" >/dev/null
printf 'sample\trss_kib\tfile_descriptors\tthreads\tdatabase_connections\n' >"$evidence_dir/resources.tsv"
printf 'resource\tgrowth\tmaximum\n' >"$evidence_dir/resource-assertions.tsv"
sample_resources before

case_index=0
run_case() {
  local scenario="$1"
  local timeout_seconds="$2"
  local expected_outcome="$3"
  case_index=$((case_index + 1))
  local label
  local prefix
  local output
  local payload
  local session_id
  label="$(printf '%02d-%s' "$case_index" "$scenario")"
  prefix="$(printf 'b6%s%02d' "$run_token" "$case_index" | tr -d '-')"
  output="$evidence_dir/$label.log"
  REVENANT_GAME_ADDR="$game_addr" \
  REVENANT_COOPERATION_ACCOUNT_PREFIX="$prefix" \
  REVENANT_COOPERATION_SCENARIO="$scenario" \
    timeout "${timeout_seconds}s" "$bot_bin" >"$output" 2>&1
  payload="$(sed -n 's/^M28_COOPERATION_BOT //p' "$output")"
  if [[ -z "$payload" ]]; then
    echo "$scenario produced no bounded bot result" >&2
    exit 1
  fi
  if [[ "$(jq -r '.scenario' <<<"$payload")" != "$scenario" ]] \
    || [[ "$(jq -r '.outcome' <<<"$payload")" != "$expected_outcome" ]]; then
    echo "$scenario returned unexpected terminal truth: $payload" >&2
    exit 1
  fi
  session_id="$(jq -r '.session_id' <<<"$payload")"
  printf '%s\t%s\t%s\n' "$scenario" "$session_id" "$expected_outcome" >>"$evidence_dir/sessions.tsv"
  curl --fail --silent --header "Origin: $inspector_origin" "http://$health_addr/api/inspector/sessions/$session_id/summary" \
    >"$evidence_dir/$label-summary.json"
  curl --fail --silent --header "Origin: $inspector_origin" "http://$health_addr/api/inspector/sessions/$session_id/events" \
    >"$evidence_dir/$label-events.json"
  if [[ "$(jq -r '.summary.cooperation_terminal_outcome' "$evidence_dir/$label-summary.json")" != "$expected_outcome" ]]; then
    echo "$scenario Inspector outcome diverged" >&2
    exit 1
  fi

  assert_sql 1 "$scenario-operation" \
    "SELECT COUNT(*) FROM cooperation_operations WHERE session_id='$session_id' AND terminal_outcome='$expected_outcome';"
  assert_sql 'anchor,runner' "$scenario-roles" \
    "SELECT string_agg(role, ',' ORDER BY participant_index) FROM cooperation_operation_participants WHERE session_id='$session_id';"
  assert_sql 0 "$scenario-route-exclusion" \
    "SELECT COUNT(*) FROM route_operations WHERE session_id='$session_id';"
  assert_sql 1 "$scenario-start-event" \
    "SELECT COUNT(*) FROM replay_events WHERE session_id='$session_id' AND event_type='cooperation_started';"
  assert_sql 1 "$scenario-terminal-event" \
    "SELECT COUNT(*) FROM replay_events WHERE session_id='$session_id' AND event_type IN ('cooperation_succeeded','cooperation_failed');"
  assert_sql 1 "$scenario-max-cooperation-event-cardinality" \
    "SELECT COALESCE(MAX(event_count),0) FROM (SELECT COUNT(*) AS event_count FROM replay_events WHERE session_id='$session_id' AND event_type IN ('cooperation_started','cooperation_pinged','player_downed','player_revived','cooperation_succeeded','cooperation_failed') GROUP BY event_type) counts;"
  if [[ "$expected_outcome" == succeeded ]]; then
    assert_sql 2 "$scenario-history" \
      "SELECT COUNT(*) FROM activity_history WHERE character_id IN (SELECT character_id FROM cooperation_operation_participants WHERE session_id='$session_id');"
    assert_sql 2 "$scenario-inventory-grants" \
      "SELECT COUNT(*) FROM inventory_reward_grants WHERE session_id='$session_id' AND item_id='relay_core_fragment' AND quantity=2;"
    assert_sql 2 "$scenario-progression-grants" \
      "SELECT COUNT(*) FROM progression_reward_grants WHERE session_id='$session_id' AND experience=125;"
  else
    assert_sql 0 "$scenario-history" \
      "SELECT COUNT(*) FROM activity_history WHERE character_id IN (SELECT character_id FROM cooperation_operation_participants WHERE session_id='$session_id');"
    assert_sql 0 "$scenario-inventory-grants" \
      "SELECT COUNT(*) FROM inventory_reward_grants WHERE session_id='$session_id';"
    assert_sql 0 "$scenario-progression-grants" \
      "SELECT COUNT(*) FROM progression_reward_grants WHERE session_id='$session_id';"
  fi
  for _ in {1..240}; do
    local reset_count
    reset_count="$(rg -c '"event":"session_reset"' "$gateway_log" || true)"
    if [[ "$reset_count" -ge "$case_index" ]]; then
      return 0
    fi
    sleep 0.05
  done
  echo "session did not reset after $scenario" >&2
  exit 1
}

run_case success 45 succeeded
run_case ping-timeout 30 failed_ping_timeout
run_case revive-timeout 40 failed_revive_timeout
run_case operation-timeout 80 failed_operation_timeout
for role in anchor runner; do
  for phase in awaiting-anchor awaiting-ping awaiting-runner runner-downed revive-channel encounter-active; do
    run_case "disconnect-$role-$phase" 35 abandoned_disconnect
  done
done

assert_sql 0 retained-public-triggers \
  "SELECT COUNT(*) FROM pg_trigger t JOIN pg_class c ON c.oid=t.tgrelid JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public' AND NOT t.tgisinternal;"
assert_sql 0 retained-public-functions \
  "SELECT COUNT(*) FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname='public';"

sample_resources after
assert_resource_growth rss-kib "$RESOURCE_RSS_BEFORE" "$RESOURCE_RSS_AFTER" 65536
assert_resource_growth file-descriptors "$RESOURCE_FDS_BEFORE" "$RESOURCE_FDS_AFTER" 4
assert_resource_growth threads "$RESOURCE_THREADS_BEFORE" "$RESOURCE_THREADS_AFTER" 4
assert_resource_growth database-connections "$RESOURCE_CONNECTIONS_BEFORE" "$RESOURCE_CONNECTIONS_AFTER" 2

if rg -q 'session_command_failed|session_abort_failed' "$gateway_log"; then
  echo "gateway reported a command or abort failure" >&2
  exit 1
fi
printf 'scenario_count\t%s\n' "$case_index" >"$evidence_dir/result.tsv"
printf 'reset_count\t%s\n' "$(rg -c '"event":"session_reset"' "$gateway_log")" >>"$evidence_dir/result.tsv"
(
  cd "$evidence_dir"
  find . -maxdepth 1 -type f ! -name SHA256SUMS -printf '%P\n' \
    | LC_ALL=C sort \
    | xargs sha256sum >SHA256SUMS
  sha256sum --check --quiet SHA256SUMS
)
printf 'M28 bot matrix passed: %s\n' "$evidence_dir"
