#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$repo_root/scripts/database-url.sh"
gateway_bin="${M26_GATEWAY_BIN:-$repo_root/target/debug/revenant-gateway}"
bot_bin="${M26_BOT_BIN:-$repo_root/target/debug/revenant-bot}"
cli_bin="${M26_CLI_BIN:-$repo_root/target/debug/revenant}"
health_addr="${M26_MATRIX_HEALTH_ADDR:-127.0.0.1:18028}"
game_addr="${M26_MATRIX_GAME_ADDR:-127.0.0.1:17028}"
inspector_origin="${REVENANT_INSPECTOR_ORIGIN:-http://127.0.0.1:4173}"
database_url="$(revenant_database_url)"
evidence_root="${M26_MATRIX_EVIDENCE_ROOT:-$repo_root/../revenant-local-evidence}"
run_token="${M26_MATRIX_RUN_TOKEN:-$(date -u +%H%M%S)-$$}"

if [[ ! "$run_token" =~ ^[a-zA-Z0-9-]{1,16}$ ]]; then
  echo "M26_MATRIX_RUN_TOKEN must contain 1-16 ASCII letters, digits, or hyphens" >&2
  exit 1
fi
for executable in "$gateway_bin" "$bot_bin" "$cli_bin"; do
  if [[ ! -x "$executable" ]]; then
    echo "M26 runtime matrix executable is unavailable: $executable" >&2
    exit 1
  fi
done
for command in curl docker jq timeout; do
  if ! command -v "$command" >/dev/null; then
    echo "M26 runtime matrix command is unavailable: $command" >&2
    exit 1
  fi
done

mkdir -p "$evidence_root"
evidence_dir="$evidence_root/m26-runtime-$run_token"
if [[ -e "$evidence_dir" ]]; then
  echo "M26 runtime matrix evidence target already exists: $evidence_dir" >&2
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
  printf '%s\t%s\t%s\n' "$label" "$expected" "$actual" \
    >>"$evidence_dir/database-assertions.tsv"
  if [[ "$actual" != "$expected" ]]; then
    echo "$label mismatch: expected $expected, got $actual" >&2
    exit 1
  fi
}

wait_for_gateway() {
  local response
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
  for _ in {1..300}; do
    if [[ "$(grep -c '"event":"session_reset"' "$gateway_log" || true)" -ge "$expected" ]]; then
      return 0
    fi
    sleep 0.05
  done
  return 1
}

start_gateway() {
  local expected_players="$1"
  gateway_log="$evidence_dir/gateway-p$expected_players.log"
  completed_runs=0
  REVENANT_BIND_ADDR="$health_addr" \
  REVENANT_GAME_ADDR="$game_addr" \
  REVENANT_EXPECTED_PLAYERS="$expected_players" \
  REVENANT_DATABASE_MODE=existing \
  DATABASE_URL="$database_url" \
    "$gateway_bin" >"$gateway_log" 2>&1 &
  gateway_pid=$!
  if ! wait_for_gateway; then
    cat "$gateway_log" >&2
    echo "M26 runtime matrix gateway did not become healthy for $expected_players players" >&2
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
  if ((rss_delta > 65536)); then
    echo "$label gateway RSS grew by more than 64 MiB: ${rss_delta} KiB" >&2
    exit 1
  fi
  if ((METRIC_FDS > before_fds + 4)); then
    echo "$label gateway retained too many descriptors: $before_fds -> $METRIC_FDS" >&2
    exit 1
  fi
  if ((METRIC_THREADS > before_threads + 4)); then
    echo "$label gateway retained too many threads: $before_threads -> $METRIC_THREADS" >&2
    exit 1
  fi
  if ((METRIC_CONNECTIONS > before_connections + 2)); then
    echo "$label retained too many PostgreSQL connections: $before_connections -> $METRIC_CONNECTIONS" >&2
    exit 1
  fi
}

run_plain_client() {
  local username="$1"
  local module_flow="$2"
  local output="$3"
  REVENANT_GAME_ADDR="$game_addr" \
  REVENANT_BOT_USERNAME="$username" \
  REVENANT_BOT_ROLE=driver \
  REVENANT_BOT_MODULE_FLOW="$module_flow" \
  REVENANT_EXPECTED_PLAYERS=1 \
    timeout 45s "$bot_bin" >"$output" 2>&1
}

run_v1_client() {
  local username="$1"
  local output="$2"
  REVENANT_GAME_ADDR="$game_addr" \
  REVENANT_BOT_USERNAME="$username" \
  REVENANT_BOT_PROTOCOL_VERSION=1 \
  REVENANT_EXPECTED_PLAYERS=1 \
    timeout 45s "$bot_bin" >"$output" 2>&1
}

run_matrix_client() {
  local username="$1"
  local role="$2"
  local weapon="$3"
  local build="$4"
  local next_build="$5"
  local operation_id="$6"
  local participants="$7"
  local output="$8"
  REVENANT_GAME_ADDR="$game_addr" \
  REVENANT_BOT_USERNAME="$username" \
  REVENANT_BOT_ROLE="$role" \
  REVENANT_BOT_WEAPON="$weapon" \
  REVENANT_BOT_EXPECT_MODULE_BUILD="$build" \
  REVENANT_BOT_NEXT_MODULE_BUILD="$next_build" \
  REVENANT_BOT_NEXT_OPERATION_ID="$operation_id" \
  REVENANT_EXPECTED_PLAYERS="$participants" \
    timeout 45s "$bot_bin" >"$output" 2>&1
}

set_profile_expectations() {
  local build="$1"
  local weapon="$2"
  local participants="$3"
  EXPECTED_MAX_HEALTH=100
  if [[ "$build" == ward || "$build" == force-ward ]]; then
    EXPECTED_MAX_HEALTH=120
  fi
  if [[ "$weapon" == pulse_rifle ]]; then
    EXPECTED_DAMAGE=40
    EXPECTED_COOLDOWN=250
    [[ "$build" == force || "$build" == force-ward ]] && EXPECTED_DAMAGE=48
    [[ "$build" == force ]] && EXPECTED_COOLDOWN=300
    [[ "$build" == ward ]] && EXPECTED_COOLDOWN=275
    [[ "$build" == force-ward ]] && EXPECTED_COOLDOWN=325
    local drone_base=140
    local warden_base=240
  else
    EXPECTED_DAMAGE=25
    EXPECTED_COOLDOWN=150
    [[ "$build" == force || "$build" == force-ward ]] && EXPECTED_DAMAGE=30
    [[ "$build" == force ]] && EXPECTED_COOLDOWN=180
    [[ "$build" == ward ]] && EXPECTED_COOLDOWN=165
    [[ "$build" == force-ward ]] && EXPECTED_COOLDOWN=195
    local drone_base=140
    local warden_base=240
  fi
  local drone_health=$((drone_base * (participants + 1) / 2))
  local warden_health=$((warden_base * (participants + 1) / 2))
  EXPECTED_DRONE_HITS=$(((drone_health + EXPECTED_DAMAGE - 1) / EXPECTED_DAMAGE))
  EXPECTED_WARDEN_HITS=$(((warden_health + EXPECTED_DAMAGE - 1) / EXPECTED_DAMAGE))
}

assert_matrix_json() {
  local json="$1"
  local weapon="$2"
  local build="$3"
  local participants="$4"
  local next_build="$5"
  set_profile_expectations "$build" "$weapon" "$participants"
  jq -e \
    --arg weapon "$weapon" \
    --arg build "$build" \
    --argjson participants "$participants" \
    --argjson maximum_health "$EXPECTED_MAX_HEALTH" \
    --argjson cooldown "$EXPECTED_COOLDOWN" \
    --argjson drone_hits "$EXPECTED_DRONE_HITS" \
    --argjson warden_hits "$EXPECTED_WARDEN_HITS" '
      .schema == "RevenantM25RuntimeMatrixV1" and
      .weapon == $weapon and .module_build == $build and
      .maximum_health == $maximum_health and .participants == $participants and
      .simulated_rtt_ms == 0 and .attack_interval_ms == $cooldown and
      .drone.accepted_hits == $drone_hits and .warden.accepted_hits == $warden_hits and
      .drone.remaining_health[-1] == 0 and .warden.remaining_health[-1] == 0 and
      .drone.hostile_damage == [10] and .warden.hostile_damage == [15, 15] and
      (.drone.hits_by_actor | length) == $participants and
      (.warden.hits_by_actor | length) == $participants and
      .loot_grants == 1 and .progression_grants == 1 and
      .experience_granted == 100 and .completed == true
    ' <<<"$json" >/dev/null
  if [[ -n "$next_build" ]]; then
    jq -e --arg next "$next_build" '
      .next_module_build == $next and
      (.next_loadout_revision | type) == "number" and
      .next_loadout_retry_replayed == true
    ' <<<"$json" >/dev/null
  else
    jq -e '
      .next_module_build == null and .next_loadout_revision == null and
      .next_loadout_retry_replayed == false
    ' <<<"$json" >/dev/null
  fi
}

resolve_session() {
  local username="$1"
  psql_value "SELECT session_id FROM replay_events WHERE account_id = 'local:$username' AND event_type = 'player_joined' ORDER BY id DESC LIMIT 1;"
}

reconcile_v2_session() {
  local username="$1"
  local secondary_username="$2"
  local participants="$3"
  local combinations="$4"
  local loadout_changes="$5"
  local label="$6"
  local session_id
  session_id="$(resolve_session "$username")"
  if [[ -z "$session_id" || ! "$session_id" =~ ^session-[0-9]+$ ]]; then
    echo "$label did not resolve a valid replay session" >&2
    exit 1
  fi
  if [[ -n "$secondary_username" && "$(resolve_session "$secondary_username")" != "$session_id" ]]; then
    echo "$label active clients resolved different replay sessions" >&2
    exit 1
  fi
  local replay_file="$evidence_dir/$label-replay.log"
  DATABASE_URL="$database_url" "$cli_bin" replay --latest "local:$username" >"$replay_file"
  grep -q "^Replay session $session_id$" "$replay_file"
  grep -q "State: activity=relay_awakening enemies_spawned=1 enemies_defeated=2 boss_spawned=true loot_grants=$participants progression_grants=$participants equipment_changes=$participants completed=true" "$replay_file"
  grep -q "Modules: legacy=false participants=$participants snapshots=$participants combinations=$combinations loadout_changes=$loadout_changes" "$replay_file"

  local summary_file="$evidence_dir/$label-summary.json"
  local events_file="$evidence_dir/$label-events.json"
  curl --fail --silent --header "Origin: $inspector_origin" "http://$health_addr/api/inspector/sessions/$session_id/summary" >"$summary_file"
  curl --fail --silent --header "Origin: $inspector_origin" "http://$health_addr/api/inspector/sessions/$session_id/events" >"$events_file"
  local expected_events=$((6 + 5 * participants + combinations + loadout_changes))
  jq -e \
    --argjson participants "$participants" \
    --argjson combinations "$combinations" \
    --argjson loadouts "$loadout_changes" \
    --argjson events "$expected_events" '
      .summary.completed == true and .summary.participant_count == $participants and
      .summary.enemy_spawn_count == 1 and .summary.enemy_defeat_count == 2 and
      .summary.boss_spawned == true and .summary.equipment_change_count == $participants and
      .summary.loot_grant_count == $participants and
      .summary.progression_grant_count == $participants and
      .summary.module_replay_legacy == false and
      .summary.module_participant_count == $participants and
      .summary.module_snapshot_count == $participants and
      .summary.module_combination_count == $combinations and
      .summary.module_loadout_change_count == $loadouts and
      .summary.event_count == $events and .summary.join_to_start_ms != null and
      .summary.activity_duration_ms != null
    ' "$summary_file" >/dev/null
  jq -e \
    --argjson participants "$participants" \
    --argjson combinations "$combinations" \
    --argjson loadouts "$loadout_changes" '
      ([.events[] | select(.event_type == "player_joined")] | length) == $participants and
      ([.events[] | select(.event_type == "module_state_snapshot")] | length) == $participants and
      ([.events[] | select(.event_type == "module_combined")] | length) == $combinations and
      ([.events[] | select(.event_type == "module_loadout_changed")] | length) == $loadouts and
      ([.events[] | select(.event_type == "activity_completed")] | length) == 1 and
      ([.events[] | select(.event_type == "enemy_died")] | length) == 2 and
      ([.events[] | select(.event_type == "boss_spawned")] | length) == 1 and
      ([.events[] | select(.event_type == "equipment_changed")] | length) == $participants and
      ([.events[] | select(.event_type == "loot_granted")] | length) == $participants and
      ([.events[] | select(.event_type == "progression_granted")] | length) == $participants
    ' "$events_file" >/dev/null
  jq -c --arg label "$label" '.summary + {case_label: $label}' "$summary_file" \
    >>"$evidence_dir/session-summaries.jsonl"
}

reconcile_v1_session() {
  local username="$1"
  local label="$2"
  local session_id
  session_id="$(resolve_session "$username")"
  local replay_file="$evidence_dir/$label-replay.log"
  DATABASE_URL="$database_url" "$cli_bin" replay --latest "local:$username" >"$replay_file"
  grep -q "^Replay session $session_id$" "$replay_file"
  grep -q 'State: activity=relay_awakening enemies_spawned=1 enemies_defeated=2 boss_spawned=true loot_grants=1 progression_grants=1 equipment_changes=0 completed=true' "$replay_file"
  grep -q 'Modules: legacy=false participants=1 snapshots=1 combinations=0 loadout_changes=0' "$replay_file"
  local summary_file="$evidence_dir/$label-summary.json"
  local events_file="$evidence_dir/$label-events.json"
  curl --fail --silent --header "Origin: $inspector_origin" "http://$health_addr/api/inspector/sessions/$session_id/summary" >"$summary_file"
  curl --fail --silent --header "Origin: $inspector_origin" "http://$health_addr/api/inspector/sessions/$session_id/events" >"$events_file"
  jq -e '
    .summary.completed == true and .summary.participant_count == 1 and
    .summary.event_count == 10 and .summary.module_replay_legacy == false and
    .summary.module_participant_count == 1 and .summary.module_snapshot_count == 1 and
    .summary.module_combination_count == 0 and .summary.module_loadout_change_count == 0
  ' "$summary_file" >/dev/null
  jq -e '
    ([.events[] | select(.event_type == "module_state_snapshot")] | length) == 1 and
    ([.events[] | select(.event_type == "module_combined")] | length) == 0 and
    ([.events[] | select(.event_type == "module_loadout_changed")] | length) == 0 and
    ([.events[] | select(.event_type == "module_state_snapshot") |
      .decoded_payload.payload.protocol_generation == "v1" and
      .decoded_payload.payload.state.applied_loadout == [] and
      .decoded_payload.payload.state.module_effects_active == false] | all)
  ' "$events_file" >/dev/null
  jq -c --arg label "$label" '.summary + {case_label: $label}' "$summary_file" \
    >>"$evidence_dir/session-summaries.jsonl"
}

finish_session() {
  completed_runs=$((completed_runs + 1))
  wait_for_resets "$completed_runs"
}

execute_solo_matrix() {
  local label="$1"
  local username="$2"
  local weapon="$3"
  local build="$4"
  local next_build="$5"
  local operation_id="$6"
  local output="$evidence_dir/$label-client.log"
  run_matrix_client "$username" matrix-primary "$weapon" "$build" "$next_build" \
    "$operation_id" 1 "$output"
  local json
  json="$(sed -n 's/^M25_MATRIX //p' "$output")"
  [[ "$(printf '%s\n' "$json" | wc -l)" -eq 1 ]]
  assert_matrix_json "$json" "$weapon" "$build" 1 "$next_build"
  jq -c --arg label "$label" '. + {case_label: $label}' <<<"$json" \
    >>"$evidence_dir/client-matrix.jsonl"
  finish_session
  local loadout_changes=0
  [[ -n "$next_build" ]] && loadout_changes=1
  reconcile_v2_session "$username" "" 1 0 "$loadout_changes" "$label"
  printf 'M26 runtime %s completed\n' "$label"
}

execute_two_active_matrix() {
  local label="$1"
  local weapon="$2"
  local primary_build="$3"
  local primary_next="$4"
  local primary_operation="$5"
  local secondary_build="$6"
  local secondary_next="$7"
  local secondary_operation="$8"
  local expected_loadouts="$9"
  local primary_output="$evidence_dir/$label-primary.log"
  local secondary_output="$evidence_dir/$label-secondary.log"
  run_matrix_client "$user_a" matrix-primary "$weapon" "$primary_build" "$primary_next" \
    "$primary_operation" 2 "$primary_output" &
  primary_pid=$!
  sleep 0.1
  run_matrix_client "$user_b" matrix-secondary "$weapon" "$secondary_build" "$secondary_next" \
    "$secondary_operation" 2 "$secondary_output" &
  secondary_pid=$!
  if ! wait "$primary_pid"; then
    primary_pid=""
    cat "$primary_output" >&2
    exit 1
  fi
  primary_pid=""
  if ! wait "$secondary_pid"; then
    secondary_pid=""
    cat "$secondary_output" >&2
    exit 1
  fi
  secondary_pid=""
  local primary_json
  local secondary_json
  primary_json="$(sed -n 's/^M25_MATRIX //p' "$primary_output")"
  secondary_json="$(sed -n 's/^M25_MATRIX //p' "$secondary_output")"
  [[ "$(printf '%s\n' "$primary_json" | wc -l)" -eq 1 ]]
  [[ "$(printf '%s\n' "$secondary_json" | wc -l)" -eq 1 ]]
  assert_matrix_json "$primary_json" "$weapon" "$primary_build" 2 "$primary_next"
  assert_matrix_json "$secondary_json" "$weapon" "$secondary_build" 2 "$secondary_next"
  local primary_shared
  local secondary_shared
  primary_shared="$(jq -S '
    del(.account_id, .role, .module_build, .maximum_health, .attack_interval_ms,
        .next_module_build, .next_loadout_revision, .next_loadout_retry_replayed) |
    del(.drone.ttk_ms, .warden.ttk_ms)
  ' <<<"$primary_json")"
  secondary_shared="$(jq -S '
    del(.account_id, .role, .module_build, .maximum_health, .attack_interval_ms,
        .next_module_build, .next_loadout_revision, .next_loadout_retry_replayed) |
    del(.drone.ttk_ms, .warden.ttk_ms)
  ' <<<"$secondary_json")"
  if [[ "$primary_shared" != "$secondary_shared" ]]; then
    echo "$label active clients projected different shared combat state" >&2
    exit 1
  fi
  jq -c --arg label "$label" '. + {case_label: $label}' <<<"$primary_json" \
    >>"$evidence_dir/client-matrix.jsonl"
  jq -c --arg label "$label" '. + {case_label: $label}' <<<"$secondary_json" \
    >>"$evidence_dir/client-matrix.jsonl"
  finish_session
  reconcile_v2_session "$user_a" "$user_b" 2 0 "$expected_loadouts" "$label"
  printf 'M26 runtime %s completed\n' "$label"
}

printf 'phase\trss_kib\tdescriptors\tthreads\tpostgres_connections\n' \
  >"$evidence_dir/resource-metrics.tsv"
printf 'assertion\texpected\tactual\n' >"$evidence_dir/database-assertions.tsv"
: >"$evidence_dir/client-matrix.jsonl"
: >"$evidence_dir/session-summaries.jsonl"
started_epoch="$(date +%s)"
username_prefix="m26x-$run_token-"
account_prefix="local:$username_prefix"
user_a="${username_prefix}a"
user_b="${username_prefix}b"
user_base_r="${username_prefix}br"
user_base_s="${username_prefix}bs"
character_a="local:$user_a:operator"
character_b="local:$user_b:operator"
assert_sql 0 preexisting-matrix-accounts \
  "SELECT count(*) FROM accounts WHERE id LIKE '$account_prefix%';"

start_gateway 1
capture_metrics p1-before
p1_before_rss="$METRIC_RSS"
p1_before_fds="$METRIC_FDS"
p1_before_threads="$METRIC_THREADS"
p1_before_connections="$METRIC_CONNECTIONS"

for username in "$user_a" "$user_b"; do
  account_code="${username##*-}"
  for run_index in 1 2 3 4; do
    label="prep-$account_code-$run_index"
    run_plain_client "$username" "" "$evidence_dir/$label-client.log"
    finish_session
    reconcile_v2_session "$username" "" 1 0 0 "$label"
  done
  label="prep-$account_code-5-mutate"
  run_plain_client "$username" mutate "$evidence_dir/$label-client.log"
  grep -q '^M26_MODULE_FLOW ' "$evidence_dir/$label-client.log"
  finish_session
  reconcile_v2_session "$username" "" 1 2 1 "$label"
done

execute_solo_matrix p1-r-empty "$user_base_r" pulse_rifle empty "" ""
execute_solo_matrix p1-r-fw "$user_a" pulse_rifle force-ward force "m26-ar1-$run_token"
execute_solo_matrix p1-r-force "$user_a" pulse_rifle force ward "m26-ar2-$run_token"
execute_solo_matrix p1-r-ward "$user_a" pulse_rifle ward force-ward "m26-ar3-$run_token"
execute_solo_matrix p1-s-empty "$user_base_s" arc_sidearm empty "" ""
execute_solo_matrix p1-s-fw "$user_a" arc_sidearm force-ward force "m26-as1-$run_token"
execute_solo_matrix p1-s-force "$user_a" arc_sidearm force ward "m26-as2-$run_token"
execute_solo_matrix p1-s-ward "$user_a" arc_sidearm ward force-ward "m26-as3-$run_token"

v1_before="$(psql_value "SELECT ms.revision || '|' || COALESCE((SELECT string_agg(mls.module_item_id, ',' ORDER BY mls.slot_index) FROM module_loadout_slots mls WHERE mls.character_id = ms.character_id), '') || '|' || (SELECT count(*) FROM module_operations mo WHERE mo.character_id = ms.character_id) FROM module_states ms WHERE ms.character_id = '$character_a';")"
run_v1_client "$user_a" "$evidence_dir/p1-v1-module-account-client.log"
grep -q '^M26_V1_PROBE ' "$evidence_dir/p1-v1-module-account-client.log"
finish_session
reconcile_v1_session "$user_a" p1-v1-module-account
v1_after="$(psql_value "SELECT ms.revision || '|' || COALESCE((SELECT string_agg(mls.module_item_id, ',' ORDER BY mls.slot_index) FROM module_loadout_slots mls WHERE mls.character_id = ms.character_id), '') || '|' || (SELECT count(*) FROM module_operations mo WHERE mo.character_id = ms.character_id) FROM module_states ms WHERE ms.character_id = '$character_a';")"
if [[ "$v1_before" != "$v1_after" ]]; then
  echo "V1 compatibility probe changed module revision, loadout, or operations" >&2
  exit 1
fi

capture_metrics p1-after
assert_resource_bounds p1 "$p1_before_rss" "$p1_before_fds" "$p1_before_threads" \
  "$p1_before_connections"
if [[ "$(grep -Ec 'session_command_failed|connection_failed|session_abort' "$gateway_log" || true)" -ne 0 ]]; then
  echo "p1 gateway logged a command, connection, or abort failure" >&2
  exit 1
fi
if [[ "$(grep -c '"event":"session_reset"' "$gateway_log" || true)" -ne 19 ]]; then
  echo "p1 gateway did not reset exactly 19 times" >&2
  exit 1
fi
stop_gateway

start_gateway 2
capture_metrics p2-before
p2_before_rss="$METRIC_RSS"
p2_before_fds="$METRIC_FDS"
p2_before_threads="$METRIC_THREADS"
p2_before_connections="$METRIC_CONNECTIONS"

execute_two_active_matrix p2-r-fw pulse_rifle force-ward force "m26-a2r1-$run_token" \
  force-ward force "m26-b2r1-$run_token" 2
execute_two_active_matrix p2-r-force pulse_rifle force ward "m26-a2r2-$run_token" \
  force ward "m26-b2r2-$run_token" 2
execute_two_active_matrix p2-r-ward pulse_rifle ward force-ward "m26-a2r3-$run_token" \
  ward force "m26-b2r3-$run_token" 2
execute_two_active_matrix p2-r-distinct pulse_rifle force-ward "" "" \
  force force-ward "m26-b2r4-$run_token" 1
execute_two_active_matrix p2-s-fw arc_sidearm force-ward force "m26-a2s1-$run_token" \
  force-ward force "m26-b2s1-$run_token" 2
execute_two_active_matrix p2-s-force arc_sidearm force ward "m26-a2s2-$run_token" \
  force ward "m26-b2s2-$run_token" 2
execute_two_active_matrix p2-s-ward arc_sidearm ward force-ward "m26-a2s3-$run_token" \
  ward force "m26-b2s3-$run_token" 2
execute_two_active_matrix p2-s-distinct arc_sidearm force-ward "" "" force "" "" 0

capture_metrics p2-after
assert_resource_bounds p2 "$p2_before_rss" "$p2_before_fds" "$p2_before_threads" \
  "$p2_before_connections"
if [[ "$(grep -Ec 'session_command_failed|connection_failed|session_abort' "$gateway_log" || true)" -ne 0 ]]; then
  echo "p2 gateway logged a command, connection, or abort failure" >&2
  exit 1
fi
if [[ "$(grep -c '"event":"session_reset"' "$gateway_log" || true)" -ne 8 ]]; then
  echo "p2 gateway did not reset exactly eight times" >&2
  exit 1
fi
stop_gateway

assert_sql 4 matrix-account-count \
  "SELECT count(*) FROM accounts WHERE id LIKE '$account_prefix%';"
assert_sql 35 matrix-history-count \
  "SELECT count(*) FROM activity_history WHERE account_id LIKE '$account_prefix%' AND activity_id = 'relay_awakening';"
assert_sql 27 matrix-fragment-total \
  "SELECT COALESCE(sum(i.quantity), 0) FROM inventory i JOIN characters c ON c.id = i.character_id WHERE c.account_id LIKE '$account_prefix%' AND i.item_id = 'relay_core_fragment';"
assert_sql 3500 matrix-experience-total \
  "SELECT COALESCE(sum(p.experience), 0) FROM progression p JOIN characters c ON c.id = p.character_id WHERE c.account_id LIKE '$account_prefix%';"
assert_sql 35 matrix-inventory-grants \
  "SELECT count(*) FROM inventory_reward_grants g JOIN characters c ON c.id = g.character_id WHERE c.account_id LIKE '$account_prefix%';"
assert_sql 35 matrix-progression-grants \
  "SELECT count(*) FROM progression_reward_grants g JOIN characters c ON c.id = g.character_id WHERE c.account_id LIKE '$account_prefix%';"
assert_sql 27 matrix-session-count \
  "SELECT count(DISTINCT session_id) FROM replay_events WHERE account_id LIKE '$account_prefix%' AND event_type = 'player_joined';"
assert_sql 2 matrix-module-state-count \
  "SELECT count(*) FROM module_states ms JOIN characters c ON c.id = ms.character_id WHERE c.account_id LIKE '$account_prefix%';"
assert_sql 4 matrix-owned-module-count \
  "SELECT count(*) FROM inventory i JOIN characters c ON c.id = i.character_id WHERE c.account_id LIKE '$account_prefix%' AND i.item_id LIKE 'module_%';"
assert_sql 3 matrix-final-loadout-slots \
  "SELECT count(*) FROM module_loadout_slots mls JOIN characters c ON c.id = mls.character_id WHERE c.account_id LIKE '$account_prefix%';"
assert_sql 4 matrix-combination-operations \
  "SELECT count(*) FROM module_operations mo JOIN characters c ON c.id = mo.character_id WHERE c.account_id LIKE '$account_prefix%' AND mo.operation_kind = 'combine';"
assert_sql 21 matrix-loadout-operations \
  "SELECT count(*) FROM module_operations mo JOIN characters c ON c.id = mo.character_id WHERE c.account_id LIKE '$account_prefix%' AND mo.operation_kind = 'loadout';"
assert_sql 21 matrix-revision-total \
  "SELECT COALESCE(sum(ms.revision), 0) FROM module_states ms JOIN characters c ON c.id = ms.character_id WHERE c.account_id LIKE '$account_prefix%';"
assert_sql 35 matrix-snapshot-events \
  "SELECT count(*) FROM replay_events WHERE account_id LIKE '$account_prefix%' AND event_type = 'module_state_snapshot';"
assert_sql 4 matrix-combination-events \
  "SELECT count(*) FROM replay_events WHERE account_id LIKE '$account_prefix%' AND event_type = 'module_combined';"
assert_sql 21 matrix-loadout-events \
  "SELECT count(*) FROM replay_events WHERE account_id LIKE '$account_prefix%' AND event_type = 'module_loadout_changed';"
assert_sql '16|13|module_force_matrix,module_ward_capacitor' matrix-final-account-a \
  "SELECT i.quantity || '|' || ms.revision || '|' || string_agg(mls.module_item_id, ',' ORDER BY mls.slot_index) FROM characters c JOIN inventory i ON i.character_id = c.id AND i.item_id = 'relay_core_fragment' JOIN module_states ms ON ms.character_id = c.id JOIN module_loadout_slots mls ON mls.character_id = c.id WHERE c.id = '$character_a' GROUP BY i.quantity, ms.revision;"
assert_sql '9|8|module_force_matrix' matrix-final-account-b \
  "SELECT i.quantity || '|' || ms.revision || '|' || string_agg(mls.module_item_id, ',' ORDER BY mls.slot_index) FROM characters c JOIN inventory i ON i.character_id = c.id AND i.item_id = 'relay_core_fragment' JOIN module_states ms ON ms.character_id = c.id JOIN module_loadout_slots mls ON mls.character_id = c.id WHERE c.id = '$character_b' GROUP BY i.quantity, ms.revision;"
assert_sql 0 matrix-session-integrity \
  "WITH selected AS (SELECT DISTINCT session_id FROM replay_events WHERE account_id LIKE '$account_prefix%' AND event_type = 'player_joined'), counts AS (SELECT r.session_id, count(*) FILTER (WHERE event_type = 'player_joined') joins, count(*) FILTER (WHERE event_type = 'activity_completed') completions, count(*) FILTER (WHERE event_type = 'loot_granted') loot, count(*) FILTER (WHERE event_type = 'progression_granted') progression, count(*) FILTER (WHERE event_type = 'equipment_changed') equipment, count(*) FILTER (WHERE event_type = 'module_state_snapshot') snapshots, count(*) FILTER (WHERE event_type = 'module_state_snapshot' AND payload::jsonb->>'protocol_generation' = 'v1') v1_snapshots, count(*) FILTER (WHERE event_type = 'enemy_died') deaths, count(*) FILTER (WHERE event_type = 'boss_spawned') bosses FROM replay_events r JOIN selected s ON s.session_id = r.session_id GROUP BY r.session_id) SELECT count(*) FROM counts WHERE joins NOT IN (1, 2) OR completions <> 1 OR loot <> joins OR progression <> joins OR snapshots <> joins OR equipment <> joins - v1_snapshots OR deaths <> 2 OR bosses <> 1;"

jq -s -e '
  length == 24 and
  ([.[] | select(.weapon == "pulse_rifle")] | length) == 12 and
  ([.[] | select(.weapon == "arc_sidearm")] | length) == 12 and
  ([.[] | select(.participants == 1)] | length) == 8 and
  ([.[] | select(.participants == 2)] | length) == 16 and
  ([.[] | select(.module_build == "empty")] | length) == 2 and
  ([.[] | select(.module_build == "force")] | length) == 8 and
  ([.[] | select(.module_build == "ward")] | length) == 6 and
  ([.[] | select(.module_build == "force-ward")] | length) == 8 and
  ([.[] | select(.case_label | endswith("distinct"))] | length) == 4
' "$evidence_dir/client-matrix.jsonl" >/dev/null
jq -s -e 'length == 27 and ([.[] | select(.module_replay_legacy == true)] | length) == 0' \
  "$evidence_dir/session-summaries.jsonl" >/dev/null
jq -s '
  group_by([.weapon, .module_build, .participants]) |
  map({
    weapon: .[0].weapon,
    module_build: .[0].module_build,
    participants: .[0].participants,
    clients: length,
    drone_hits: (map(.drone.accepted_hits) | unique),
    warden_hits: (map(.warden.accepted_hits) | unique),
    attack_intervals_ms: (map(.attack_interval_ms) | unique),
    maximum_health: (map(.maximum_health) | unique)
  })
' "$evidence_dir/client-matrix.jsonl" >"$evidence_dir/coverage-summary.json"

finished_epoch="$(date +%s)"
duration_seconds=$((finished_epoch - started_epoch))
printf 'run_token=%s\nsessions=27\nactive_client_completions=35\naccounts=4\nmodule_accounts=2\npulse_rifle_matrix_clients=12\narc_sidearm_matrix_clients=12\nsolo_matrix_clients=8\ntwo_active_matrix_clients=16\nv1_module_account_sessions=1\nduration_seconds=%s\n' \
  "$run_token" "$duration_seconds" >"$evidence_dir/summary.txt"

(
  cd "$evidence_dir"
  find . -type f ! -name SHA256SUMS -print0 | sort -z | xargs -0 sha256sum >SHA256SUMS
  sha256sum --check --quiet SHA256SUMS
)

printf 'M26 runtime matrix passed: %s\n' "$evidence_dir"
