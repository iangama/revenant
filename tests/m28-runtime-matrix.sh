#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$repo_root/scripts/database-url.sh"
gateway_bin="${M28_RUNTIME_GATEWAY_BIN:-$repo_root/target/debug/revenant-gateway}"
bot_bin="${M28_RUNTIME_BOT_BIN:-$repo_root/target/debug/revenant-bot}"
cooperation_bot_bin="${M28_RUNTIME_COOPERATION_BOT_BIN:-$repo_root/target/debug/revenant-cooperation-bot}"
cli_bin="${M28_RUNTIME_CLI_BIN:-$repo_root/target/debug/revenant}"
frozen_bin="${M28_RUNTIME_FROZEN_BIN:-$repo_root/target/debug/revenant-client-v1}"
lab_bin="${M28_RUNTIME_LAB_BIN:-$repo_root/target/debug/revenant-cooperation-lab}"
health_addr="${M28_RUNTIME_HEALTH_ADDR:-127.0.0.1:18028}"
game_addr="${M28_RUNTIME_GAME_ADDR:-127.0.0.1:17028}"
inspector_origin="${REVENANT_INSPECTOR_ORIGIN:-http://127.0.0.1:4173}"
database_url="$(revenant_database_url)"
evidence_root="${M28_RUNTIME_EVIDENCE_ROOT:-$repo_root/../revenant-local-evidence}"
run_token="${M28_RUNTIME_RUN_TOKEN:-$(date -u +%m%d%H%M)-$$}"

if [[ ! "$run_token" =~ ^[a-zA-Z0-9-]{1,8}$ ]]; then
  echo "M28_RUNTIME_RUN_TOKEN must contain 1-8 ASCII letters, digits, or hyphens" >&2
  exit 1
fi
for executable in "$gateway_bin" "$bot_bin" "$cooperation_bot_bin" "$cli_bin" "$frozen_bin" "$lab_bin"; do
  if [[ ! -x "$executable" ]]; then
    echo "M28 runtime matrix executable is unavailable: $executable" >&2
    exit 1
  fi
done
for command in awk cmp curl docker find jq rg sha256sum ss timeout; do
  if ! command -v "$command" >/dev/null; then
    echo "M28 runtime matrix command is unavailable: $command" >&2
    exit 1
  fi
done

mkdir -p "$evidence_root"
evidence_dir="$evidence_root/m28-block8-runtime-$run_token"
if [[ -e "$evidence_dir" ]]; then
  echo "M28 runtime matrix evidence target already exists: $evidence_dir" >&2
  exit 1
fi
mkdir "$evidence_dir"

gateway_pid=""
primary_pid=""
secondary_pid=""
gateway_log=""
completed_runs=0

cleanup() {
  for process_id in "$primary_pid" "$secondary_pid" "$gateway_pid"; do
    if [[ -n "$process_id" ]]; then
      kill "$process_id" 2>/dev/null || true
      wait "$process_id" 2>/dev/null || true
    fi
  done
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
  printf '%s\t%s\t%s\n' "$label" "$expected" "$actual" >>"$evidence_dir/database-assertions.tsv"
  if [[ "$actual" != "$expected" ]]; then
    echo "$label mismatch: expected $expected, got $actual" >&2
    exit 1
  fi
}

wait_for_gateway() {
  for _ in {1..180}; do
    if curl --fail --silent "http://$health_addr/health" | rg -q '"status":"ok"'; then
      return 0
    fi
    if ! kill -0 "$gateway_pid" 2>/dev/null; then
      return 1
    fi
    sleep 0.1
  done
  return 1
}

wait_for_resets() {
  local expected="$1"
  for _ in {1..500}; do
    if [[ "$(rg -c '"event":"session_reset"' "$gateway_log" || true)" -ge "$expected" ]]; then
      return 0
    fi
    sleep 0.05
  done
  return 1
}

capture_resources() {
  local label="$1"
  local rss_kib
  local descriptors
  local threads
  local connections
  rss_kib="$(awk '/^VmRSS:/ {print $2}' "/proc/$gateway_pid/status")"
  descriptors="$(find "/proc/$gateway_pid/fd" -mindepth 1 -maxdepth 1 -type l | wc -l)"
  threads="$(find "/proc/$gateway_pid/task" -mindepth 1 -maxdepth 1 -type d | wc -l)"
  connections="$(psql_value "SELECT count(*) FROM pg_stat_activity WHERE datname='revenant' AND usename='revenant';")"
  printf '%s\t%s\t%s\t%s\t%s\n' "$label" "$rss_kib" "$descriptors" "$threads" "$connections" \
    >>"$evidence_dir/resources.tsv"
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
  local rss_growth=$((METRIC_RSS - before_rss))
  ((rss_growth < 0)) && rss_growth=0
  local fd_growth=$((METRIC_FDS - before_fds))
  ((fd_growth < 0)) && fd_growth=0
  local thread_growth=$((METRIC_THREADS - before_threads))
  ((thread_growth < 0)) && thread_growth=0
  local connection_growth=$((METRIC_CONNECTIONS - before_connections))
  ((connection_growth < 0)) && connection_growth=0
  printf '%s-rss-kib\t%s\t65536\n' "$label" "$rss_growth" >>"$evidence_dir/resource-assertions.tsv"
  printf '%s-file-descriptors\t%s\t4\n' "$label" "$fd_growth" >>"$evidence_dir/resource-assertions.tsv"
  printf '%s-threads\t%s\t4\n' "$label" "$thread_growth" >>"$evidence_dir/resource-assertions.tsv"
  printf '%s-database-connections\t%s\t2\n' "$label" "$connection_growth" >>"$evidence_dir/resource-assertions.tsv"
  if ((rss_growth > 65536 || fd_growth > 4 || thread_growth > 4 || connection_growth > 2)); then
    echo "$label gateway exceeded a retained resource bound" >&2
    exit 1
  fi
}

start_gateway() {
  local expected_players="$1"
  local label="$2"
  gateway_log="$evidence_dir/gateway-$label.log"
  completed_runs=0
  REVENANT_BIND_ADDR="$health_addr" \
  REVENANT_GAME_ADDR="$game_addr" \
  REVENANT_EXPECTED_PLAYERS="$expected_players" \
  REVENANT_DATABASE_MODE=existing \
  DATABASE_URL="$database_url" \
    "$gateway_bin" >"$gateway_log" 2>&1 &
  gateway_pid=$!
  if ! wait_for_gateway; then
    sed -n '1,260p' "$gateway_log" >&2
    echo "M28 runtime gateway did not become healthy for $label" >&2
    exit 1
  fi
  capture_resources "$label-before"
  GROUP_RSS_BEFORE="$METRIC_RSS"
  GROUP_FDS_BEFORE="$METRIC_FDS"
  GROUP_THREADS_BEFORE="$METRIC_THREADS"
  GROUP_CONNECTIONS_BEFORE="$METRIC_CONNECTIONS"
}

finish_session() {
  completed_runs=$((completed_runs + 1))
  if ! wait_for_resets "$completed_runs"; then
    echo "gateway did not reset after session $completed_runs" >&2
    exit 1
  fi
}

stop_gateway() {
  local label="$1"
  local expected_resets="$2"
  capture_resources "$label-after"
  assert_resource_bounds "$label" "$GROUP_RSS_BEFORE" "$GROUP_FDS_BEFORE" \
    "$GROUP_THREADS_BEFORE" "$GROUP_CONNECTIONS_BEFORE"
  local actual_resets
  actual_resets="$(rg -c '"event":"session_reset"' "$gateway_log" || true)"
  if [[ "$actual_resets" != "$expected_resets" ]]; then
    echo "$label expected $expected_resets resets, got $actual_resets" >&2
    exit 1
  fi
  if rg -q 'session_command_failed|connection_failed|session_abort_failed|panicked at|seed_injection' "$gateway_log"; then
    echo "$label gateway logged an unexpected failure or test seed hook" >&2
    exit 1
  fi
  kill "$gateway_pid"
  wait "$gateway_pid" 2>/dev/null || true
  gateway_pid=""
}

resolve_session() {
  local username="$1"
  psql_value "SELECT session_id FROM replay_events WHERE account_id='local:$username' AND event_type='player_joined' ORDER BY id DESC LIMIT 1;"
}

capture_session_artifacts() {
  local session_id="$1"
  local label="$2"
  DATABASE_URL="$database_url" "$cli_bin" replay "$session_id" >"$evidence_dir/$label-replay.log"
  curl --fail --silent --header "Origin: $inspector_origin" "http://$health_addr/api/inspector/sessions/$session_id/summary" \
    >"$evidence_dir/$label-summary.json"
  curl --fail --silent --header "Origin: $inspector_origin" "http://$health_addr/api/inspector/sessions/$session_id/events" \
    >"$evidence_dir/$label-events.json"
  printf '%s\t%s\n' "$label" "$session_id" >>"$evidence_dir/session-ids.tsv"
}

record_latest_session() {
  local username="$1"
  local label="$2"
  local session_id
  session_id="$(resolve_session "$username")"
  if [[ ! "$session_id" =~ ^session-[0-9]+$ ]]; then
    echo "$label did not resolve a bounded session" >&2
    exit 1
  fi
  printf '%s\t%s\n' "$label" "$session_id" >>"$evidence_dir/preparation-sessions.tsv"
}

run_plain_client() {
  local username="$1"
  local weapon="$2"
  local module_flow="$3"
  local output="$4"
  REVENANT_GAME_ADDR="$game_addr" \
  REVENANT_BOT_USERNAME="$username" \
  REVENANT_BOT_ROLE=driver \
  REVENANT_BOT_WEAPON="$weapon" \
  REVENANT_BOT_MODULE_FLOW="$module_flow" \
  REVENANT_EXPECTED_PLAYERS=1 \
    timeout 45s "$bot_bin" >"$output" 2>&1
}

run_loadout_transition() {
  local username="$1"
  local weapon="$2"
  local next_build="$3"
  local operation_id="$4"
  local output="$5"
  REVENANT_GAME_ADDR="$game_addr" \
  REVENANT_BOT_USERNAME="$username" \
  REVENANT_BOT_ROLE=matrix-primary \
  REVENANT_BOT_WEAPON="$weapon" \
  REVENANT_BOT_EXPECT_MODULE_BUILD=force-ward \
  REVENANT_BOT_NEXT_MODULE_BUILD="$next_build" \
  REVENANT_BOT_NEXT_OPERATION_ID="$operation_id" \
  REVENANT_EXPECTED_PLAYERS=1 \
    timeout 45s "$bot_bin" >"$output" 2>&1
  local result
  result="$(sed -n 's/^M25_MATRIX //p' "$output")"
  jq -e --arg next "$next_build" --arg weapon "$weapon" '
    .schema == "RevenantM25RuntimeMatrixV1" and .completed == true and
    .module_build == "force-ward" and .weapon == $weapon and
    .next_module_build == $next and .next_loadout_retry_replayed == true
  ' <<<"$result" >/dev/null
}

prepare_account() {
  local username="$1"
  local final_build="$2"
  local weapon="$3"
  local code="$4"
  for run_index in 1 2 3 4; do
    local label="prep-$code-$run_index"
    run_plain_client "$username" "$weapon" "" "$evidence_dir/$label-client.log"
    finish_session
    record_latest_session "$username" "$label"
  done
  local mutation_label="prep-$code-5-mutate"
  run_plain_client "$username" "$weapon" mutate "$evidence_dir/$mutation_label-client.log"
  rg -q '^M26_MODULE_FLOW ' "$evidence_dir/$mutation_label-client.log"
  finish_session
  record_latest_session "$username" "$mutation_label"
  if [[ "$final_build" != force-ward ]]; then
    local transition_label="prep-$code-6-$final_build"
    run_loadout_transition "$username" "$weapon" "$final_build" \
      "b8-$run_token-$code" "$evidence_dir/$transition_label-client.log"
    finish_session
    record_latest_session "$username" "$transition_label"
  fi
}

run_current_v1() {
  local username="$1"
  local output="$2"
  REVENANT_GAME_ADDR="$game_addr" \
  REVENANT_BOT_USERNAME="$username" \
  REVENANT_BOT_PROTOCOL_VERSION=1 \
  REVENANT_EXPECTED_PLAYERS=1 \
    timeout 45s "$bot_bin" >"$output" 2>&1
  rg -q '^M26_V1_PROBE ' "$output"
}

profile_assertion() {
  local session_id="$1"
  local role="$2"
  local weapon="$3"
  local loadout="$4"
  local damage="$5"
  local range="$6"
  local cooldown="$7"
  local maximum_health="$8"
  local label="$9"
  assert_sql 1 "$label" "SELECT count(*) FROM cooperation_operation_participants WHERE session_id='$session_id' AND role='$role' AND weapon_item_id='$weapon' AND module_loadout='$loadout'::jsonb AND damage=$damage AND range=$range AND cooldown_ms=$cooldown AND max_health=$maximum_health AND current_health > 0 AND life_state='active';"
}

run_build_success() {
  local pair_prefix="$1"
  local label="$2"
  shift 2
  local output="$evidence_dir/$label-client.log"
  REVENANT_GAME_ADDR="$game_addr" \
  REVENANT_COOPERATION_ACCOUNT_PREFIX="$pair_prefix" \
  REVENANT_COOPERATION_SCENARIO=success \
    timeout 45s "$cooperation_bot_bin" >"$output" 2>&1
  local result
  local session_id
  result="$(sed -n 's/^M28_COOPERATION_BOT //p' "$output")"
  jq -e '
    .schema == "RevenantM28CooperationBotV1" and .scenario == "success" and
    .outcome == "succeeded" and .subject_role == null and .grant_count == 2 and
    .contributions.anchor_arrived and .contributions.pinged and
    .contributions.runner_arrived and .contributions.revived and
    .contributions.warden_completed and .contributions.revive_count == 1
  ' <<<"$result" >/dev/null
  session_id="$(jq -r '.session_id' <<<"$result")"
  finish_session
  capture_session_artifacts "$session_id" "$label"
  jq -e '
    .summary.completed == true and .summary.participant_count == 2 and
    .summary.route_replay_legacy == true and .summary.route_id == null and
    .summary.cooperation_replay_legacy == false and
    .summary.cooperation_replay_state == "succeeded" and
    .summary.cooperation_terminal_outcome == "succeeded" and
    .summary.cooperation_participant_count == 2 and
    .summary.cooperation_contribution_count == 5 and
    .summary.cooperation_revive_count == 1 and
    .summary.cooperation_reward_participant_count == 2 and
    .summary.loot_grant_count == 2 and .summary.progression_grant_count == 2
  ' "$evidence_dir/$label-summary.json" >/dev/null
  jq -e '
    ([.events[] | select(.event_type == "cooperation_started")] | length) == 1 and
    ([.events[] | select(.event_type == "cooperation_pinged")] | length) == 1 and
    ([.events[] | select(.event_type == "player_downed")] | length) == 1 and
    ([.events[] | select(.event_type == "player_revived")] | length) == 1 and
    ([.events[] | select(.event_type == "cooperation_succeeded")] | length) == 1 and
    ([.events[] | select(.event_type == "cooperation_failed" or .event_type == "route_selected")] | length) == 0 and
    ([.events[] | select(.event_type == "loot_granted")] | length) == 2 and
    ([.events[] | select(.event_type == "progression_granted")] | length) == 2
  ' "$evidence_dir/$label-events.json" >/dev/null
  assert_sql 1 "$label-operation" "SELECT count(*) FROM cooperation_operations WHERE session_id='$session_id' AND terminal_outcome='succeeded' AND anchor_arrived AND pinged AND runner_arrived AND revived AND revive_count=1 AND warden_completed;"
  assert_sql 0 "$label-route-exclusion" "SELECT count(*) FROM route_operations WHERE session_id='$session_id';"
  profile_assertion "$session_id" anchor "$1" "$2" "$3" "$4" "$5" "$6" "$label-anchor-profile"
  profile_assertion "$session_id" runner "$7" "$8" "$9" "${10}" "${11}" "${12}" "$label-runner-profile"
  printf '%s\t%s\t%s\t%s\n' "$label" "$session_id" "$1/$2" "$7/$8" >>"$evidence_dir/build-successes.tsv"
  LAST_COOPERATION_SESSION="$session_id"
}

run_two_player_baseline() {
  local driver="$1"
  local observer="$2"
  local label="$3"
  REVENANT_GAME_ADDR="$game_addr" REVENANT_BOT_USERNAME="$observer" \
  REVENANT_BOT_ROLE=observer REVENANT_EXPECTED_PLAYERS=2 \
    timeout 45s "$bot_bin" >"$evidence_dir/$label-observer.log" 2>&1 &
  secondary_pid=$!
  sleep 0.2
  REVENANT_GAME_ADDR="$game_addr" REVENANT_BOT_USERNAME="$driver" \
  REVENANT_BOT_ROLE=driver REVENANT_EXPECTED_PLAYERS=2 \
    timeout 45s "$bot_bin" >"$evidence_dir/$label-driver.log" 2>&1 &
  primary_pid=$!
  if ! wait "$primary_pid"; then
    primary_pid=""
    sed -n '1,260p' "$evidence_dir/$label-driver.log" >&2
    exit 1
  fi
  primary_pid=""
  if ! wait "$secondary_pid"; then
    secondary_pid=""
    sed -n '1,260p' "$evidence_dir/$label-observer.log" >&2
    exit 1
  fi
  secondary_pid=""
  finish_session
  local session_id
  session_id="$(resolve_session "$driver")"
  [[ "$session_id" == "$(resolve_session "$observer")" ]]
  capture_session_artifacts "$session_id" "$label"
  jq -e '
    .summary.completed == true and .summary.participant_count == 2 and
    .summary.loot_grant_count == 2 and .summary.progression_grant_count == 2 and
    .summary.route_replay_legacy == true and .summary.route_id == null and
    .summary.cooperation_replay_legacy == true and
    .summary.cooperation_replay_state == "legacy" and
    .summary.cooperation_terminal_outcome == null and
    .summary.cooperation_participant_count == 0
  ' "$evidence_dir/$label-summary.json" >/dev/null
  jq -e '
    ([.events[] | select(
      (.event_type | startswith("cooperation_")) or
      (.event_type | startswith("player_down")) or
      (.event_type | startswith("player_reviv")) or
      (.event_type | startswith("route_"))
    )] | length) == 0
  ' "$evidence_dir/$label-events.json" >/dev/null
  assert_sql 0 "$label-cooperation-row" "SELECT count(*) FROM cooperation_operations WHERE session_id='$session_id';"
  assert_sql 0 "$label-route-row" "SELECT count(*) FROM route_operations WHERE session_id='$session_id';"
}

printf 'assertion\texpected\tactual\n' >"$evidence_dir/database-assertions.tsv"
printf 'label\tsession_id\n' >"$evidence_dir/session-ids.tsv"
printf 'label\tsession_id\n' >"$evidence_dir/preparation-sessions.tsv"
printf 'label\tsession_id\tanchor_profile\trunner_profile\n' >"$evidence_dir/build-successes.tsv"
printf 'sample\trss_kib\tfile_descriptors\tthreads\tdatabase_connections\n' >"$evidence_dir/resources.tsv"
printf 'resource\tgrowth\tmaximum\n' >"$evidence_dir/resource-assertions.tsv"
started_epoch="$(date +%s)"

"$lab_bin" --json >"$evidence_dir/cooperation-lab-before.json"
"$lab_bin" --domain-json >"$evidence_dir/cooperation-domain-before.json"
jq -e '.schema == "RevenantCooperationLabV1" and (.matrix_rows | length) == 900 and .invariants.every_row_complete and .invariants.every_row_nonlethal' "$evidence_dir/cooperation-lab-before.json" >/dev/null
jq -e '.schema == "RevenantCooperationDomainLabV1" and (.cases | length) == 50' "$evidence_dir/cooperation-domain-before.json" >/dev/null

username_prefix="m28x-$run_token-"
account_prefix="local:$username_prefix"
pair_1="${username_prefix}s1"
pair_2="${username_prefix}s2"
pair_3="${username_prefix}s3"
pair_4="${username_prefix}s4"
ordinary_driver="${username_prefix}od"
ordinary_observer="${username_prefix}oo"
route_user="${username_prefix}rt"
v1_user="${username_prefix}v1"

assert_sql 0 preexisting-runtime-accounts "SELECT count(*) FROM accounts WHERE id LIKE '$account_prefix%';"

start_gateway 1 solo
prepare_account "${pair_1}-r" force-ward arc_sidearm s1r
prepare_account "${pair_2}-a" force arc_sidearm s2a
prepare_account "${pair_2}-r" ward pulse_rifle s2r
prepare_account "${pair_3}-a" ward pulse_rifle s3a
prepare_account "${pair_3}-r" force arc_sidearm s3r
prepare_account "${pair_4}-a" force-ward arc_sidearm s4a

REVENANT_GAME_ADDR="$game_addr" REVENANT_BOT_USERNAME="$route_user" \
REVENANT_BOT_ROLE=driver REVENANT_BOT_ROUTE=breach REVENANT_BOT_WEAPON=arc_sidearm \
REVENANT_EXPECTED_PLAYERS=1 timeout 45s "$bot_bin" >"$evidence_dir/compat-route-client.log" 2>&1
rg -q '^M27_ROUTE_FLOW ' "$evidence_dir/compat-route-client.log"
finish_session
route_session="$(resolve_session "$route_user")"
capture_session_artifacts "$route_session" compat-route
jq -e '
  .summary.completed == true and .summary.participant_count == 1 and
  .summary.route_replay_legacy == false and .summary.route_id == "breach" and
  .summary.route_terminal_outcome == "succeeded" and
  .summary.route_reward_participant_count == 1 and
  .summary.cooperation_replay_legacy == true and
  .summary.cooperation_terminal_outcome == null
' "$evidence_dir/compat-route-summary.json" >/dev/null
assert_sql 1 compat-route-row "SELECT count(*) FROM route_operations WHERE session_id='$route_session' AND route_id='breach' AND terminal_outcome='succeeded';"
assert_sql 0 compat-route-cooperation-row "SELECT count(*) FROM cooperation_operations WHERE session_id='$route_session';"

run_current_v1 "$v1_user" "$evidence_dir/compat-current-v1-client.log"
finish_session
v1_session="$(resolve_session "$v1_user")"
capture_session_artifacts "$v1_session" compat-current-v1
jq -e '
  .summary.completed == true and .summary.participant_count == 1 and
  .summary.loot_grant_count == 1 and .summary.progression_grant_count == 1 and
  .summary.route_replay_legacy == true and .summary.cooperation_replay_legacy == true
' "$evidence_dir/compat-current-v1-summary.json" >/dev/null
assert_sql 0 compat-current-v1-route "SELECT count(*) FROM route_operations WHERE session_id='$v1_session';"
assert_sql 0 compat-current-v1-cooperation "SELECT count(*) FROM cooperation_operations WHERE session_id='$v1_session';"

frozen_before="$(resolve_session revenant-frozen-v1)"
REVENANT_GAME_ADDR="$game_addr" timeout 15s "$frozen_bin" >"$evidence_dir/compat-frozen-v1-client.log" 2>&1
rg -q 'frozen client V1 build 0.1.0-frozen' "$evidence_dir/compat-frozen-v1-client.log"
finish_session
frozen_session="$(resolve_session revenant-frozen-v1)"
if [[ -z "$frozen_session" || "$frozen_session" == "$frozen_before" ]]; then
  echo "frozen V1 did not create a fresh compatibility session" >&2
  exit 1
fi
capture_session_artifacts "$frozen_session" compat-frozen-v1
jq -e '
  .summary.completed == false and .summary.participant_count == 1 and
  .summary.route_replay_legacy == true and .summary.cooperation_replay_legacy == true and
  .summary.route_id == null and .summary.cooperation_terminal_outcome == null
' "$evidence_dir/compat-frozen-v1-summary.json" >/dev/null
assert_sql 0 compat-frozen-v1-route "SELECT count(*) FROM route_operations WHERE session_id='$frozen_session';"
assert_sql 0 compat-frozen-v1-cooperation "SELECT count(*) FROM cooperation_operations WHERE session_id='$frozen_session';"
stop_gateway solo 37

start_gateway 2 build
run_build_success "$pair_1" build-success-01 \
  pulse_rifle '[]' 40 6 250 100 \
  arc_sidearm '["module_force_matrix","module_ward_capacitor"]' 30 8 195 120
first_cooperation_session="$LAST_COOPERATION_SESSION"
run_build_success "$pair_2" build-success-02 \
  arc_sidearm '["module_force_matrix"]' 30 8 180 100 \
  pulse_rifle '["module_ward_capacitor"]' 40 6 275 120
run_build_success "$pair_3" build-success-03 \
  pulse_rifle '["module_ward_capacitor"]' 40 6 275 120 \
  arc_sidearm '["module_force_matrix"]' 30 8 180 100
run_build_success "$pair_4" build-success-04 \
  arc_sidearm '["module_force_matrix","module_ward_capacitor"]' 30 8 195 120 \
  pulse_rifle '[]' 40 6 250 100

run_two_player_baseline "$ordinary_driver" "$ordinary_observer" compat-ordinary-v2

for method in GET HEAD POST PUT PATCH DELETE OPTIONS; do
  code="$(curl --silent --output /dev/null --write-out '%{http_code}' --request "$method" \
    --header "Origin: $inspector_origin" "http://$health_addr/api/inspector/sessions/$first_cooperation_session/summary")"
  printf '%s\t%s\n' "$method" "$code" >>"$evidence_dir/inspector-methods.tsv"
done
if [[ "$(sed -n '1p' "$evidence_dir/inspector-methods.tsv")" != $'GET\t200' ]] \
  || [[ "$(tail -n +2 "$evidence_dir/inspector-methods.tsv" | cut -f2 | sort -u)" != 405 ]]; then
  echo "Inspector method boundary diverged" >&2
  exit 1
fi
stop_gateway build 5

M28_BOT_EVIDENCE_ROOT="$evidence_dir" \
M28_BOT_RUN_TOKEN="${run_token}b" \
M28_BOT_HEALTH_ADDR=127.0.0.1:18089 \
M28_BOT_GAME_ADDR=127.0.0.1:17089 \
DATABASE_URL="$database_url" \
  "$repo_root/tests/m28-bot-matrix.sh" | tee "$evidence_dir/bot-matrix-driver.log"

CARGO_HOME="$repo_root/.tooling/cargo" \
RUSTUP_HOME="$repo_root/.tooling/rustup" \
PATH="$repo_root/.tooling/cargo/bin:$PATH" \
  cargo test -p revenant-gateway cooperation_participant_defeat_commits_before_life_and_summary -- --nocapture \
  | tee "$evidence_dir/participant-defeat-gateway-test.log"
CARGO_HOME="$repo_root/.tooling/cargo" \
RUSTUP_HOME="$repo_root/.tooling/rustup" \
PATH="$repo_root/.tooling/cargo/bin:$PATH" \
DATABASE_URL="$database_url" \
  cargo test -p revenant-persistence --test cooperation_postgres every_failure_family_is_terminal_and_reward_free -- --nocapture \
  | tee "$evidence_dir/participant-defeat-postgres-test.log"
CARGO_HOME="$repo_root/.tooling/cargo" \
RUSTUP_HOME="$repo_root/.tooling/rustup" \
PATH="$repo_root/.tooling/cargo/bin:$PATH" \
DATABASE_URL="$database_url" \
  cargo test -p revenant-persistence --test cooperation_replay_postgres every_failure_family_reconstructs_with_exact_subject_and_zero_rewards -- --nocapture \
  | tee "$evidence_dir/participant-defeat-replay-test.log"

assert_sql 12 runtime-account-count "SELECT count(*) FROM accounts WHERE id LIKE '$account_prefix%';"
assert_sql 41 runtime-session-count "SELECT count(DISTINCT session_id) FROM replay_events WHERE account_id LIKE '$account_prefix%' AND event_type='player_joined';"
assert_sql 46 runtime-participant-joins "SELECT count(*) FROM replay_events WHERE account_id LIKE '$account_prefix%' AND event_type='player_joined';"
assert_sql 46 runtime-history-count "SELECT count(*) FROM activity_history WHERE account_id LIKE '$account_prefix%' AND activity_id='relay_awakening';"
assert_sql 46 runtime-inventory-grant-rows "SELECT count(*) FROM inventory_reward_grants g JOIN characters c ON c.id=g.character_id WHERE c.account_id LIKE '$account_prefix%';"
assert_sql 55 runtime-fragments-awarded "SELECT coalesce(sum(g.quantity),0) FROM inventory_reward_grants g JOIN characters c ON c.id=g.character_id WHERE c.account_id LIKE '$account_prefix%';"
assert_sql 46 runtime-progression-grant-rows "SELECT count(*) FROM progression_reward_grants g JOIN characters c ON c.id=g.character_id WHERE c.account_id LIKE '$account_prefix%';"
assert_sql 4800 runtime-experience-total "SELECT coalesce(sum(p.experience),0) FROM progression p JOIN characters c ON c.id=p.character_id WHERE c.account_id LIKE '$account_prefix%';"
assert_sql 31 runtime-current-fragments "SELECT coalesce(sum(i.quantity),0) FROM inventory i JOIN characters c ON c.id=i.character_id WHERE c.account_id LIKE '$account_prefix%' AND i.item_id='relay_core_fragment';"
assert_sql 4 runtime-cooperation-rows "SELECT count(*) FROM cooperation_operations WHERE anchor_account_id LIKE '$account_prefix%' AND terminal_outcome='succeeded';"
assert_sql 8 runtime-cooperation-participants "SELECT count(*) FROM cooperation_operation_participants cp JOIN cooperation_operations co USING(session_id) WHERE co.anchor_account_id LIKE '$account_prefix%';"
assert_sql 1 runtime-route-rows "SELECT count(*) FROM route_operations WHERE leader_account_id LIKE '$account_prefix%' AND terminal_outcome='succeeded';"
assert_sql 6 runtime-module-states "SELECT count(*) FROM module_states ms JOIN characters c ON c.id=ms.character_id WHERE c.account_id LIKE '$account_prefix%';"
assert_sql 12 runtime-owned-modules "SELECT count(*) FROM inventory i JOIN characters c ON c.id=i.character_id WHERE c.account_id LIKE '$account_prefix%' AND i.item_id LIKE 'module_%';"
assert_sql 8 runtime-loadout-slots "SELECT count(*) FROM module_loadout_slots ml JOIN characters c ON c.id=ml.character_id WHERE c.account_id LIKE '$account_prefix%';"
assert_sql 12 runtime-combination-operations "SELECT count(*) FROM module_operations mo JOIN characters c ON c.id=mo.character_id WHERE c.account_id LIKE '$account_prefix%' AND mo.operation_kind='combine';"
assert_sql 10 runtime-loadout-operations "SELECT count(*) FROM module_operations mo JOIN characters c ON c.id=mo.character_id WHERE c.account_id LIKE '$account_prefix%' AND mo.operation_kind='loadout';"
assert_sql 10 runtime-loadout-revisions "SELECT coalesce(sum(ms.revision),0) FROM module_states ms JOIN characters c ON c.id=ms.character_id WHERE c.account_id LIKE '$account_prefix%';"
assert_sql 0 runtime-cooperation-integrity "WITH evidence AS (SELECT co.session_id, co.terminal_outcome, co.participant_count, count(*) FILTER (WHERE re.event_type='cooperation_started') starts, count(*) FILTER (WHERE re.event_type='cooperation_pinged') pings, count(*) FILTER (WHERE re.event_type='player_downed') downs, count(*) FILTER (WHERE re.event_type='player_revived') revives, count(*) FILTER (WHERE re.event_type='cooperation_succeeded') successes, count(*) FILTER (WHERE re.event_type='cooperation_failed') failures, count(*) FILTER (WHERE re.event_type='activity_completed') completions, count(*) FILTER (WHERE re.event_type='loot_granted') loot, count(*) FILTER (WHERE re.event_type='progression_granted') progression FROM cooperation_operations co LEFT JOIN replay_events re USING(session_id) WHERE co.anchor_account_id LIKE '$account_prefix%' GROUP BY co.session_id, co.terminal_outcome, co.participant_count) SELECT count(*) FROM evidence WHERE terminal_outcome <> 'succeeded' OR starts<>1 OR pings<>1 OR downs<>1 OR revives<>1 OR successes<>1 OR failures<>0 OR completions<>1 OR loot<>participant_count OR progression<>participant_count;"
assert_sql 0 retained-public-triggers "SELECT count(*) FROM pg_trigger t JOIN pg_class c ON c.oid=t.tgrelid JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public' AND NOT t.tgisinternal;"
assert_sql 0 retained-public-functions "SELECT count(*) FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname='public';"

"$lab_bin" --json >"$evidence_dir/cooperation-lab-after.json"
"$lab_bin" --domain-json >"$evidence_dir/cooperation-domain-after.json"
cmp -s "$evidence_dir/cooperation-lab-before.json" "$evidence_dir/cooperation-lab-after.json"
cmp -s "$evidence_dir/cooperation-domain-before.json" "$evidence_dir/cooperation-domain-after.json"
matrix_hash="$(sha256sum "$evidence_dir/cooperation-lab-before.json" | cut -d' ' -f1)"
domain_hash="$(sha256sum "$evidence_dir/cooperation-domain-before.json" | cut -d' ' -f1)"
[[ "$matrix_hash" == 513de2fb226458d28d46d945f650ae482d18ad388d4f1ab8d30d2e8ae830ceb5 ]]
[[ "$domain_hash" == 34fa219abb14a98e642a622615d17b0f5d7c71f7a7c796853b751cde3c06c261 ]]
printf 'matrix\t%s\ndomain\t%s\n' "$matrix_hash" "$domain_hash" >"$evidence_dir/lab-hashes.tsv"

main_hash="$(sha256sum "$repo_root/archive/clients/v1/src/main.rs" | cut -d' ' -f1)"
cargo_hash="$(sha256sum "$repo_root/archive/clients/v1/Cargo.toml" | cut -d' ' -f1)"
[[ "$main_hash" == 4f481e9fc5d22a5ab6d8f2d0a40e2d05dc9aaf92099debdd9dedf59c26f31f72 ]]
[[ "$cargo_hash" == c951c5fe88daa2dd9fb91a4da98ca316fd3923e0bff5332d748db44bce367322 ]]
printf 'src/main.rs\t%s\nCargo.toml\t%s\n' "$main_hash" "$cargo_hash" >"$evidence_dir/frozen-v1-hashes.tsv"

docker compose -f "$repo_root/infra/docker-compose.yml" ps >"$evidence_dir/permanent-services.txt"
if docker compose -f "$repo_root/infra/docker-compose.yml" ps | rg -qv '(^NAME|healthy|postgres|gateway|inspector|^$)'; then
  echo "permanent Compose services are not healthy" >&2
  exit 1
fi
if ss -ltn | rg ':17028|:18028|:17089|:18089'; then
  echo "an isolated M28 listener remained after the matrix" >&2
  exit 1
fi

finished_epoch="$(date +%s)"
duration_seconds=$((finished_epoch - started_epoch))
printf 'run_token=%s\nprefixed_accounts=12\nprefixed_sessions=41\nparticipant_joins=46\ncompleted_histories=46\nbuild_success_sessions=4\nbot_matrix_sessions=16\ncompatibility_sessions=4\nduration_seconds=%s\n' \
  "$run_token" "$duration_seconds" >"$evidence_dir/summary.txt"

(
  cd "$evidence_dir"
  find . -type f ! -name SHA256SUMS -print0 | sort -z | xargs -0 sha256sum >SHA256SUMS
  sha256sum --check --quiet SHA256SUMS
)

printf 'M28 runtime matrix passed: %s\n' "$evidence_dir"
