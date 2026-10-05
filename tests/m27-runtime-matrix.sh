#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$repo_root/scripts/database-url.sh"
gateway_bin="${M27_GATEWAY_BIN:-$repo_root/target/debug/revenant-m27-matrix-gateway}"
bot_bin="${M27_BOT_BIN:-$repo_root/target/debug/revenant-bot}"
cli_bin="${M27_CLI_BIN:-$repo_root/target/debug/revenant}"
frozen_bin="${M27_FROZEN_BIN:-$repo_root/target/debug/revenant-client-v1}"
operation_lab_bin="${M27_OPERATION_LAB_BIN:-$repo_root/target/debug/revenant-operation-lab}"
health_addr="${M27_MATRIX_HEALTH_ADDR:-127.0.0.1:18029}"
game_addr="${M27_MATRIX_GAME_ADDR:-127.0.0.1:17029}"
inspector_origin="${REVENANT_INSPECTOR_ORIGIN:-http://127.0.0.1:4173}"
database_url="$(revenant_database_url)"
evidence_root="${M27_MATRIX_EVIDENCE_ROOT:-$repo_root/../revenant-local-evidence}"
run_token="${M27_MATRIX_RUN_TOKEN:-$(date -u +%m%d%H%M)-$$}"

if [[ ! "$run_token" =~ ^[a-zA-Z0-9-]{1,10}$ ]]; then
  echo "M27_MATRIX_RUN_TOKEN must contain 1-10 ASCII letters, digits, or hyphens" >&2
  exit 1
fi
for executable in "$gateway_bin" "$bot_bin" "$cli_bin" "$frozen_bin" "$operation_lab_bin"; do
  if [[ ! -x "$executable" ]]; then
    echo "M27 runtime matrix executable is unavailable: $executable" >&2
    exit 1
  fi
done
for command in curl docker jq timeout sha256sum; do
  if ! command -v "$command" >/dev/null; then
    echo "M27 runtime matrix command is unavailable: $command" >&2
    exit 1
  fi
done

mkdir -p "$evidence_root"
evidence_dir="$evidence_root/m27-runtime-$run_token"
if [[ -e "$evidence_dir" ]]; then
  echo "M27 runtime matrix evidence target already exists: $evidence_dir" >&2
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
  docker compose -f "$repo_root/infra/docker-compose.yml" exec -T postgres psql -U revenant -d revenant -v ON_ERROR_STOP=1 -Atc "$1"
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
  for _ in {1..400}; do
    if [[ "$(grep -c '"event":"session_reset"' "$gateway_log" || true)" -ge "$expected" ]]; then
      return 0
    fi
    sleep 0.05
  done
  return 1
}

start_gateway() {
  local expected_players="$1"
  local seeds="$2"
  local label="$3"
  gateway_log="$evidence_dir/gateway-$label.log"
  completed_runs=0
  REVENANT_BIND_ADDR="$health_addr" REVENANT_GAME_ADDR="$game_addr" REVENANT_EXPECTED_PLAYERS="$expected_players" REVENANT_M27_MATRIX_SEEDS="$seeds" REVENANT_DATABASE_MODE=existing DATABASE_URL="$database_url" "$gateway_bin" >"$gateway_log" 2>&1 &
  gateway_pid=$!
  if ! wait_for_gateway; then
    sed -n '1,240p' "$gateway_log" >&2
    echo "M27 runtime matrix gateway did not become healthy for $label" >&2
    exit 1
  fi
  grep -q '"event":"m27_matrix_seed_injection_enabled"' "$gateway_log"
}

stop_gateway() {
  kill "$gateway_pid"
  wait "$gateway_pid" 2>/dev/null || true
  gateway_pid=""
}

finish_session() {
  completed_runs=$((completed_runs + 1))
  wait_for_resets "$completed_runs"
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
  printf '%s\t%s\t%s\t%s\t%s\n' "$phase" "$rss_kib" "$descriptors" "$threads" "$connections" >>"$evidence_dir/resource-metrics.tsv"
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
  REVENANT_GAME_ADDR="$game_addr" REVENANT_BOT_USERNAME="$username" REVENANT_BOT_ROLE=driver REVENANT_BOT_MODULE_FLOW="$module_flow" REVENANT_EXPECTED_PLAYERS=1 timeout 45s "$bot_bin" >"$output" 2>&1
}

run_v1_client() {
  local username="$1"
  local output="$2"
  REVENANT_GAME_ADDR="$game_addr" REVENANT_BOT_USERNAME="$username" REVENANT_BOT_PROTOCOL_VERSION=1 REVENANT_EXPECTED_PLAYERS=1 timeout 45s "$bot_bin" >"$output" 2>&1
}

run_route_matrix_client() {
  local username="$1"
  local role="$2"
  local route="$3"
  local weapon="$4"
  local build="$5"
  local next_build="$6"
  local operation_id="$7"
  local participants="$8"
  local output="$9"
  REVENANT_GAME_ADDR="$game_addr" REVENANT_BOT_USERNAME="$username" REVENANT_BOT_ROLE="$role" REVENANT_BOT_ROUTE="$route" REVENANT_BOT_WEAPON="$weapon" REVENANT_BOT_EXPECT_MODULE_BUILD="$build" REVENANT_BOT_NEXT_MODULE_BUILD="$next_build" REVENANT_BOT_NEXT_OPERATION_ID="$operation_id" REVENANT_EXPECTED_PLAYERS="$participants" timeout 45s "$bot_bin" >"$output" 2>&1
}

run_route_probe() {
  local username="$1"
  local route="$2"
  local mode="$3"
  local weapon="$4"
  local output="$5"
  local timeout_seconds=45
  [[ "$mode" == timeout ]] && timeout_seconds=110
  REVENANT_GAME_ADDR="$game_addr" REVENANT_BOT_USERNAME="$username" REVENANT_BOT_ROLE=driver REVENANT_BOT_ROUTE="$route" REVENANT_BOT_ROUTE_MODE="$mode" REVENANT_BOT_WEAPON="$weapon" REVENANT_EXPECTED_PLAYERS=1 timeout "${timeout_seconds}s" "$bot_bin" >"$output" 2>&1
}

resolve_session() {
  local username="$1"
  psql_value "SELECT session_id FROM replay_events WHERE account_id = 'local:$username' AND event_type = 'player_joined' ORDER BY id DESC LIMIT 1;"
}

capture_session_artifacts() {
  local session_id="$1"
  local label="$2"
  DATABASE_URL="$database_url" "$cli_bin" replay "$session_id" >"$evidence_dir/$label-replay.log"
  curl --fail --silent --header "Origin: $inspector_origin" "http://$health_addr/api/inspector/sessions/$session_id/summary" >"$evidence_dir/$label-summary.json"
  curl --fail --silent --header "Origin: $inspector_origin" "http://$health_addr/api/inspector/sessions/$session_id/events" >"$evidence_dir/$label-events.json"
  printf '%s\t%s\n' "$label" "$session_id" >>"$evidence_dir/session-ids.tsv"
}

reconcile_baseline() {
  local username="$1"
  local secondary_username="$2"
  local participants="$3"
  local equipment_changes="$4"
  local combinations="$5"
  local loadout_changes="$6"
  local label="$7"
  local session_id
  session_id="$(resolve_session "$username")"
  if [[ -z "$session_id" || ! "$session_id" =~ ^session-[0-9]+$ ]]; then
    echo "$label did not resolve a valid replay session" >&2
    exit 1
  fi
  if [[ -n "$secondary_username" && "$(resolve_session "$secondary_username")" != "$session_id" ]]; then
    echo "$label active clients resolved different sessions" >&2
    exit 1
  fi
  capture_session_artifacts "$session_id" "$label"
  local expected_events=$((6 + 4 * participants + equipment_changes + combinations + loadout_changes))
  grep -q "State: activity=relay_awakening enemies_spawned=1 enemies_defeated=2 boss_spawned=true loot_grants=$participants progression_grants=$participants equipment_changes=$equipment_changes completed=true" "$evidence_dir/$label-replay.log"
  grep -q "Modules: legacy=false participants=$participants snapshots=$participants combinations=$combinations loadout_changes=$loadout_changes" "$evidence_dir/$label-replay.log"
  jq -e --argjson participants "$participants" --argjson equipment "$equipment_changes" --argjson combinations "$combinations" --argjson loadouts "$loadout_changes" --argjson events "$expected_events" '
      .summary.completed == true and .summary.participant_count == $participants and
      .summary.enemy_spawn_count == 1 and .summary.enemy_defeat_count == 2 and
      .summary.boss_spawned == true and .summary.equipment_change_count == $equipment and
      .summary.loot_grant_count == $participants and
      .summary.progression_grant_count == $participants and
      .summary.module_participant_count == $participants and
      .summary.module_snapshot_count == $participants and
      .summary.module_combination_count == $combinations and
      .summary.module_loadout_change_count == $loadouts and
      .summary.route_replay_legacy == true and .summary.route_id == null and
      .summary.route_event_id == null and .summary.route_terminal_outcome == null and
      .summary.route_elapsed_ms == null and .summary.route_transition_count == 0 and
      .summary.route_reward_participant_count == 0 and .summary.event_count == $events
    ' "$evidence_dir/$label-summary.json" >/dev/null
  jq -e '
    ([.events[] | select(.event_type == "route_selected" or
      .event_type == "route_operation_succeeded" or
      .event_type == "route_operation_failed")] | length) == 0
    ' "$evidence_dir/$label-events.json" >/dev/null
  assert_sql 0 "$label-route-row" "SELECT count(*) FROM route_operations WHERE session_id = '$session_id';"
  jq -c --arg label "$label" '.summary + {case_label: $label}' "$evidence_dir/$label-summary.json" >>"$evidence_dir/session-summaries.jsonl"
}

set_route_expectations() {
  local route="$1"
  local event="$2"
  local seed="$3"
  local weapon="$4"
  local build="$5"
  local participants="$6"
  EXPECTED_MAX_HEALTH=100
  if [[ "$build" == ward || "$build" == force-ward ]]; then
    EXPECTED_MAX_HEALTH=120
  fi
  if [[ "$weapon" == pulse_rifle ]]; then
    EXPECTED_BASE_DAMAGE=40
    EXPECTED_FORCE_DAMAGE=48
    EXPECTED_DAMAGE=40
    EXPECTED_COOLDOWN=250
    [[ "$build" == force || "$build" == force-ward ]] && EXPECTED_DAMAGE=48
    [[ "$build" == force ]] && EXPECTED_COOLDOWN=300
    [[ "$build" == ward ]] && EXPECTED_COOLDOWN=275
    [[ "$build" == force-ward ]] && EXPECTED_COOLDOWN=325
  else
    EXPECTED_BASE_DAMAGE=25
    EXPECTED_FORCE_DAMAGE=30
    EXPECTED_DAMAGE=25
    EXPECTED_COOLDOWN=150
    [[ "$build" == force || "$build" == force-ward ]] && EXPECTED_DAMAGE=30
    [[ "$build" == force ]] && EXPECTED_COOLDOWN=180
    [[ "$build" == ward ]] && EXPECTED_COOLDOWN=165
    [[ "$build" == force-ward ]] && EXPECTED_COOLDOWN=195
  fi
  case "$route/$event/$seed" in
    breach/overcharged_armor/0)
      EXPECTED_HEALTH_BP=12000
      EXPECTED_COUNTER=15
      EXPECTED_REWARD_QUANTITY=2
      EXPECTED_REWARD_XP=100
      EXPECTED_TRANSITIONS=5
      ;;
    breach/arc_surge/1)
      EXPECTED_HEALTH_BP=10000
      EXPECTED_COUNTER=20
      EXPECTED_REWARD_QUANTITY=2
      EXPECTED_REWARD_XP=100
      EXPECTED_TRANSITIONS=5
      ;;
    stabilize/shielded_channel/0)
      EXPECTED_HEALTH_BP=10000
      EXPECTED_COUNTER=10
      EXPECTED_REWARD_QUANTITY=1
      EXPECTED_REWARD_XP=150
      EXPECTED_TRANSITIONS=7
      ;;
    stabilize/residual_feedback/2)
      EXPECTED_HEALTH_BP=11000
      EXPECTED_COUNTER=15
      EXPECTED_REWARD_QUANTITY=1
      EXPECTED_REWARD_XP=150
      EXPECTED_TRANSITIONS=7
      ;;
    *)
      echo "unreviewed M27 matrix route/event/seed: $route/$event/$seed" >&2
      exit 1
      ;;
  esac
  local drone_health=$((140 * (participants + 1) / 2))
  local warden_base=$((240 * (participants + 1) / 2))
  EXPECTED_DRONE_HEALTH="$drone_health"
  EXPECTED_WARDEN_HEALTH=$(((warden_base * EXPECTED_HEALTH_BP + 5000) / 10000))
}

assert_route_json() {
  local json="$1"
  local role="$2"
  local route="$3"
  local event="$4"
  local seed="$5"
  local weapon="$6"
  local build="$7"
  local participants="$8"
  local next_build="$9"
  set_route_expectations "$route" "$event" "$seed" "$weapon" "$build" "$participants"
  jq -e --arg role "$role" --arg route "$route" --arg event "$event" --arg weapon "$weapon" --arg build "$build" --argjson seed "$seed" --argjson participants "$participants" --argjson maximum_health "$EXPECTED_MAX_HEALTH" --argjson cooldown "$EXPECTED_COOLDOWN" --argjson damage "$EXPECTED_DAMAGE" --argjson base_damage "$EXPECTED_BASE_DAMAGE" --argjson force_damage "$EXPECTED_FORCE_DAMAGE" --argjson health_bp "$EXPECTED_HEALTH_BP" --argjson counter "$EXPECTED_COUNTER" --argjson drone_health "$EXPECTED_DRONE_HEALTH" --argjson warden_health "$EXPECTED_WARDEN_HEALTH" --argjson quantity "$EXPECTED_REWARD_QUANTITY" --argjson experience "$EXPECTED_REWARD_XP" --argjson transitions "$EXPECTED_TRANSITIONS" '
      .schema == "RevenantM27RuntimeMatrixV1" and .role == $role and
      .route_id == $route and .event_id == $event and .seed == $seed and
      .weapon == $weapon and .module_build == $build and
      .maximum_health == $maximum_health and .participants == $participants and
      .attack_interval_ms == $cooldown and
      .warden_health_basis_points == $health_bp and
      .warden_counter_damage == $counter and .duration_budget_ms == 90000 and
      .elapsed_ms <= .duration_budget_ms and .transition_count == $transitions and
      (.objective_path | length) == (if $route == "breach" then 3 else 4 end) and
      .drone.target_health == $drone_health and .warden.target_health == $warden_health and
      .drone.accepted_hits == (.drone.remaining_health | length) and
      .drone.accepted_hits == (.drone.accepted_damage | length) and
      .drone.accepted_hits == (.drone.hit_sources | length) and
      .warden.accepted_hits == (.warden.remaining_health | length) and
      .warden.accepted_hits == (.warden.accepted_damage | length) and
      .warden.accepted_hits == (.warden.hit_sources | length) and
      (.drone.accepted_damage | all(. == $base_damage or . == $force_damage)) and
      (.warden.accepted_damage | all(. == $base_damage or . == $force_damage)) and
      (if $participants == 1 then
        (.drone.accepted_damage | all(. == $damage)) and
        (.warden.accepted_damage | all(. == $damage))
       else true end) and
      (.drone.accepted_damage | add) >= .drone.target_health and
      ((.drone.accepted_damage | add) - .drone.accepted_damage[-1]) < .drone.target_health and
      (.warden.accepted_damage | add) >= .warden.target_health and
      ((.warden.accepted_damage | add) - .warden.accepted_damage[-1]) < .warden.target_health and
      .drone.remaining_health[-1] == 0 and .warden.remaining_health[-1] == 0 and
      .drone.hostile_damage == [10] and .warden.hostile_damage == [$counter, $counter] and
      (.drone.hits_by_actor | length) == $participants and
      (.warden.hits_by_actor | length) == $participants and
      (.drone.hit_sources | unique | length) == $participants and
      (.warden.hit_sources | unique | length) == $participants and
      .loot_quantity == $quantity and .experience_granted == $experience and
      .terminal_grant_count == $participants and .completed == true and
      (if $role == "primary" then
        .retry_replayed == true and .conflict_rejected == true and
        .non_leader_rejected == false
       else
        .retry_replayed == false and .conflict_rejected == false and
        .non_leader_rejected == true
       end)
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

reconcile_route_success() {
  local username="$1"
  local secondary_username="$2"
  local participants="$3"
  local loadout_changes="$4"
  local label="$5"
  local json="$6"
  local session_id
  session_id="$(resolve_session "$username")"
  if [[ -z "$session_id" || ! "$session_id" =~ ^session-[0-9]+$ ]]; then
    echo "$label did not resolve a valid routed session" >&2
    exit 1
  fi
  if [[ -n "$secondary_username" && "$(resolve_session "$secondary_username")" != "$session_id" ]]; then
    echo "$label active route clients resolved different sessions" >&2
    exit 1
  fi
  capture_session_artifacts "$session_id" "$label"
  local route
  local event
  local seed
  local elapsed
  local transitions
  route="$(jq -r '.route_id' <<<"$json")"
  event="$(jq -r '.event_id' <<<"$json")"
  seed="$(jq -r '.seed' <<<"$json")"
  elapsed="$(jq -r '.elapsed_ms' <<<"$json")"
  transitions="$(jq -r '.transition_count' <<<"$json")"
  local expected_events=$((8 + 5 * participants + loadout_changes))
  grep -q '| route_selected |' "$evidence_dir/$label-replay.log"
  grep -q '| route_operation_succeeded |' "$evidence_dir/$label-replay.log"
  grep -q "State: activity=relay_awakening enemies_spawned=1 enemies_defeated=2 boss_spawned=true loot_grants=$participants progression_grants=$participants equipment_changes=$participants completed=true" "$evidence_dir/$label-replay.log"
  grep -q "Modules: legacy=false participants=$participants snapshots=$participants combinations=0 loadout_changes=$loadout_changes" "$evidence_dir/$label-replay.log"
  jq -e --arg route "$route" --arg event "$event" --argjson participants "$participants" --argjson elapsed "$elapsed" --argjson transitions "$transitions" --argjson events "$expected_events" '
      .summary.completed == true and .summary.participant_count == $participants and
      .summary.enemy_spawn_count == 1 and .summary.enemy_defeat_count == 2 and
      .summary.boss_spawned == true and .summary.equipment_change_count == $participants and
      .summary.loot_grant_count == $participants and
      .summary.progression_grant_count == $participants and
      .summary.route_replay_legacy == false and .summary.route_id == $route and
      .summary.route_event_id == $event and
      .summary.route_terminal_outcome == "succeeded" and
      .summary.route_elapsed_ms == $elapsed and
      .summary.route_transition_count == $transitions and
      .summary.route_reward_participant_count == $participants and
      .summary.event_count == $events
    ' "$evidence_dir/$label-summary.json" >/dev/null
  jq -e --argjson participants "$participants" --argjson loadouts "$loadout_changes" '
      ([.events[] | select(.event_type == "route_selected")] | length) == 1 and
      ([.events[] | select(.event_type == "route_operation_succeeded")] | length) == 1 and
      ([.events[] | select(.event_type == "route_operation_failed")] | length) == 0 and
      ([.events[] | select(.event_type == "activity_completed")] | length) == 1 and
      ([.events[] | select(.event_type == "loot_granted")] | length) == $participants and
      ([.events[] | select(.event_type == "progression_granted")] | length) == $participants and
      ([.events[] | select(.event_type == "module_loadout_changed")] | length) == $loadouts
    ' "$evidence_dir/$label-events.json" >/dev/null
  assert_sql "1|$route|$event|$seed|succeeded|$elapsed|$participants" "$label-route-row" "SELECT count(*) || '|' || min(route_id) || '|' || min(event_id) || '|' || min(seed) || '|' || min(terminal_outcome) || '|' || min(elapsed_ms) || '|' || min(participant_count) FROM route_operations WHERE session_id = '$session_id';"
  assert_sql "$participants" "$label-route-participants" "SELECT count(*) FROM route_operation_participants WHERE session_id = '$session_id';"
  jq -c --arg label "$label" '. + {case_label: $label}' <<<"$json" >>"$evidence_dir/client-matrix.jsonl"
  jq -c --arg label "$label" '.summary + {case_label: $label}' "$evidence_dir/$label-summary.json" >>"$evidence_dir/session-summaries.jsonl"
}

execute_solo_route() {
  local label="$1"
  local username="$2"
  local route="$3"
  local event="$4"
  local seed="$5"
  local weapon="$6"
  local build="$7"
  local next_build="$8"
  local operation_id="$9"
  local output="$evidence_dir/$label-client.log"
  run_route_matrix_client "$username" matrix-primary "$route" "$weapon" "$build" "$next_build" "$operation_id" 1 "$output"
  local json
  json="$(sed -n 's/^M27_MATRIX //p' "$output")"
  [[ "$(printf '%s\n' "$json" | wc -l)" -eq 1 ]]
  assert_route_json "$json" primary "$route" "$event" "$seed" "$weapon" "$build" 1 "$next_build"
  finish_session
  local loadout_changes=0
  [[ -n "$next_build" ]] && loadout_changes=1
  reconcile_route_success "$username" "" 1 "$loadout_changes" "$label" "$json"
  printf 'M27 runtime %s completed\n' "$label"
}

execute_two_active_route() {
  local label="$1"
  local route="$2"
  local event="$3"
  local seed="$4"
  local weapon="$5"
  local primary_username="$6"
  local primary_build="$7"
  local primary_next="$8"
  local primary_operation="$9"
  shift 9
  local secondary_username="$1"
  local secondary_build="$2"
  local secondary_next="$3"
  local secondary_operation="$4"
  local expected_loadouts="$5"
  local primary_output="$evidence_dir/$label-primary.log"
  local secondary_output="$evidence_dir/$label-secondary.log"
  run_route_matrix_client "$primary_username" matrix-primary "$route" "$weapon" "$primary_build" "$primary_next" "$primary_operation" 2 "$primary_output" &
  primary_pid=$!
  sleep 0.1
  run_route_matrix_client "$secondary_username" matrix-secondary "$route" "$weapon" "$secondary_build" "$secondary_next" "$secondary_operation" 2 "$secondary_output" &
  secondary_pid=$!
  if ! wait "$primary_pid"; then
    primary_pid=""
    sed -n '1,280p' "$primary_output" >&2
    exit 1
  fi
  primary_pid=""
  if ! wait "$secondary_pid"; then
    secondary_pid=""
    sed -n '1,280p' "$secondary_output" >&2
    exit 1
  fi
  secondary_pid=""
  local primary_json
  local secondary_json
  primary_json="$(sed -n 's/^M27_MATRIX //p' "$primary_output")"
  secondary_json="$(sed -n 's/^M27_MATRIX //p' "$secondary_output")"
  [[ "$(printf '%s\n' "$primary_json" | wc -l)" -eq 1 ]]
  [[ "$(printf '%s\n' "$secondary_json" | wc -l)" -eq 1 ]]
  assert_route_json "$primary_json" primary "$route" "$event" "$seed" "$weapon" "$primary_build" 2 "$primary_next"
  assert_route_json "$secondary_json" secondary "$route" "$event" "$seed" "$weapon" "$secondary_build" 2 "$secondary_next"
  local primary_shared
  local secondary_shared
  primary_shared="$(jq -S 'del(.account_id, .role, .module_build, .maximum_health, .attack_interval_ms, .retry_replayed, .conflict_rejected, .non_leader_rejected, .next_module_build, .next_loadout_revision, .next_loadout_retry_replayed) | del(.drone.ttk_ms, .warden.ttk_ms)' <<<"$primary_json")"
  secondary_shared="$(jq -S 'del(.account_id, .role, .module_build, .maximum_health, .attack_interval_ms, .retry_replayed, .conflict_rejected, .non_leader_rejected, .next_module_build, .next_loadout_revision, .next_loadout_retry_replayed) | del(.drone.ttk_ms, .warden.ttk_ms)' <<<"$secondary_json")"
  if [[ "$primary_shared" != "$secondary_shared" ]]; then
    echo "$label active clients projected different shared route truth" >&2
    exit 1
  fi
  finish_session
  reconcile_route_success "$primary_username" "$secondary_username" 2 "$expected_loadouts" "$label" "$primary_json"
  jq -c --arg label "$label-secondary" '. + {case_label: $label}' <<<"$secondary_json" >>"$evidence_dir/client-matrix.jsonl"
  printf 'M27 runtime %s completed\n' "$label"
}

reconcile_disconnect() {
  local username="$1"
  local label="$2"
  local json="$3"
  local session_id
  session_id="$(resolve_session "$username")"
  capture_session_artifacts "$session_id" "$label"
  jq -e '
    .schema == "RevenantM27RouteDisconnectV1" and
    .route_id == "stabilize" and .event_id == "shielded_channel" and
    .seed == 0 and .selected == true and .terminal == false
  ' <<<"$json" >/dev/null
  grep -q '| route_selected |' "$evidence_dir/$label-replay.log"
  if grep -Eq 'route_operation_(succeeded|failed)|activity_completed|loot_granted|progression_granted' "$evidence_dir/$label-replay.log"; then
    echo "$label replay fabricated a terminal or reward" >&2
    exit 1
  fi
  jq -e '
    .summary.completed == false and .summary.participant_count == 1 and
    .summary.enemy_spawn_count == 1 and .summary.enemy_defeat_count == 1 and
    .summary.boss_spawned == false and .summary.equipment_change_count == 1 and
    .summary.loot_grant_count == 0 and .summary.progression_grant_count == 0 and
    .summary.route_replay_legacy == false and .summary.route_id == "stabilize" and
    .summary.route_event_id == "shielded_channel" and
    .summary.route_terminal_outcome == null and .summary.route_elapsed_ms == null and
    .summary.route_transition_count == 0 and
    .summary.route_reward_participant_count == 0 and .summary.event_count == 7
  ' "$evidence_dir/$label-summary.json" >/dev/null
  assert_sql '1|stabilize|shielded_channel|0||1' "$label-route-row" "SELECT count(*) || '|' || min(route_id) || '|' || min(event_id) || '|' || min(seed) || '|' || COALESCE(min(terminal_outcome), '') || '|' || min(participant_count) FROM route_operations WHERE session_id = '$session_id';"
  assert_sql 0 "$label-history" "SELECT count(*) FROM activity_history WHERE account_id = 'local:$username';"
  assert_sql 0 "$label-terminal-replay" "SELECT count(*) FROM replay_events WHERE session_id = '$session_id' AND event_type IN ('route_operation_succeeded', 'route_operation_failed');"
  jq -c --arg label "$label" '.summary + {case_label: $label}' "$evidence_dir/$label-summary.json" >>"$evidence_dir/session-summaries.jsonl"
}

reconcile_timeout() {
  local username="$1"
  local label="$2"
  local json="$3"
  local session_id
  session_id="$(resolve_session "$username")"
  capture_session_artifacts "$session_id" "$label"
  local elapsed
  local transitions
  elapsed="$(jq -r '.elapsed_ms' <<<"$json")"
  transitions="$(jq -r '.transition_count' <<<"$json")"
  jq -e '
    .schema == "RevenantM27RouteTimeoutV1" and
    .route_id == "breach" and .event_id == "arc_surge" and .seed == 1 and
    .duration_budget_ms == 90000 and .elapsed_ms > .duration_budget_ms and
    .failed_objective == "reach_relay_door" and
    .completed == false and .rewarded == false
  ' <<<"$json" >/dev/null
  grep -q '| route_selected |' "$evidence_dir/$label-replay.log"
  grep -q '| route_operation_failed |' "$evidence_dir/$label-replay.log"
  if grep -Eq 'route_operation_succeeded|activity_completed|loot_granted|progression_granted' "$evidence_dir/$label-replay.log"; then
    echo "$label replay fabricated success or reward" >&2
    exit 1
  fi
  jq -e --argjson elapsed "$elapsed" --argjson transitions "$transitions" '
    .summary.completed == false and .summary.participant_count == 1 and
    .summary.enemy_spawn_count == 1 and .summary.enemy_defeat_count == 1 and
    .summary.boss_spawned == false and .summary.equipment_change_count == 1 and
    .summary.loot_grant_count == 0 and .summary.progression_grant_count == 0 and
    .summary.route_replay_legacy == false and .summary.route_id == "breach" and
    .summary.route_event_id == "arc_surge" and
    .summary.route_terminal_outcome == "failed_timeout" and
    .summary.route_elapsed_ms == $elapsed and
    .summary.route_transition_count == $transitions and
    .summary.route_reward_participant_count == 0 and .summary.event_count == 8
  ' "$evidence_dir/$label-summary.json" >/dev/null
  assert_sql "1|breach|arc_surge|1|failed_timeout|$elapsed|1" "$label-route-row" "SELECT count(*) || '|' || min(route_id) || '|' || min(event_id) || '|' || min(seed) || '|' || min(terminal_outcome) || '|' || min(elapsed_ms) || '|' || min(participant_count) FROM route_operations WHERE session_id = '$session_id';"
  assert_sql 0 "$label-history" "SELECT count(*) FROM activity_history WHERE account_id = 'local:$username';"
  assert_sql 1 "$label-failure-replay" "SELECT count(*) FROM replay_events WHERE session_id = '$session_id' AND event_type = 'route_operation_failed';"
  jq -c --arg label "$label" '.summary + {case_label: $label}' "$evidence_dir/$label-summary.json" >>"$evidence_dir/session-summaries.jsonl"
}

reconcile_frozen_v1() {
  local session_id="$1"
  local label="$2"
  capture_session_artifacts "$session_id" "$label"
  jq -e '
    .summary.completed == false and .summary.participant_count == 1 and
    .summary.module_participant_count == 1 and .summary.module_snapshot_count == 1 and
    .summary.route_replay_legacy == true and .summary.route_id == null and
    .summary.route_event_id == null and .summary.route_terminal_outcome == null and
    .summary.route_reward_participant_count == 0 and
    .summary.event_count >= 2 and .summary.event_count <= 4
  ' "$evidence_dir/$label-summary.json" >/dev/null
  jq -e '
    ([.events[] | select(.event_type == "route_selected" or
      .event_type == "route_operation_succeeded" or
      .event_type == "route_operation_failed")] | length) == 0 and
    ([.events[] | select(.event_type == "module_state_snapshot" and
      .decoded_payload.payload.protocol_generation == "v1" and
      .decoded_payload.payload.state.applied_loadout == [] and
      .decoded_payload.payload.state.module_effects_active == false)] | length) == 1
  ' "$evidence_dir/$label-events.json" >/dev/null
  assert_sql 0 "$label-route-row" "SELECT count(*) FROM route_operations WHERE session_id = '$session_id';"
}

execute_capability_mismatch() {
  local label="$1"
  local leader="$2"
  local peer="$3"
  local leader_output="$evidence_dir/$label-leader.log"
  local peer_output="$evidence_dir/$label-peer.log"
  REVENANT_GAME_ADDR="$game_addr" REVENANT_BOT_USERNAME="$leader" REVENANT_BOT_ROLE=driver REVENANT_BOT_ROUTE=breach REVENANT_BOT_ROUTE_MODE=capability-mismatch REVENANT_BOT_WEAPON=arc_sidearm REVENANT_EXPECTED_PLAYERS=2 timeout 45s "$bot_bin" >"$leader_output" 2>&1 &
  primary_pid=$!
  sleep 0.1
  REVENANT_GAME_ADDR="$game_addr" REVENANT_BOT_USERNAME="$peer" REVENANT_BOT_ROLE=observer REVENANT_EXPECTED_PLAYERS=2 timeout 45s "$bot_bin" >"$peer_output" 2>&1 &
  secondary_pid=$!
  if ! wait "$primary_pid"; then
    primary_pid=""
    sed -n '1,300p' "$leader_output" >&2
    exit 1
  fi
  primary_pid=""
  if ! wait "$secondary_pid"; then
    secondary_pid=""
    sed -n '1,300p' "$peer_output" >&2
    exit 1
  fi
  secondary_pid=""
  grep -q '^M27_ROUTE_CAPABILITY_MISMATCH ' "$leader_output"
  finish_session
  reconcile_baseline "$leader" "$peer" 2 1 0 0 "$label"
}

printf 'phase\trss_kib\tdescriptors\tthreads\tpostgres_connections\n' >"$evidence_dir/resource-metrics.tsv"
printf 'assertion\texpected\tactual\n' >"$evidence_dir/database-assertions.tsv"
printf 'case_label\tsession_id\n' >"$evidence_dir/session-ids.tsv"
: >"$evidence_dir/client-matrix.jsonl"
: >"$evidence_dir/session-summaries.jsonl"
started_epoch="$(date +%s)"

"$operation_lab_bin" --json >"$evidence_dir/operation-lab-before.json"
"$operation_lab_bin" --domain-json >"$evidence_dir/operation-domain-before.json"
jq -e '
  .schema == "RevenantOperationLabV1" and
  (.routed_rows | length) == 240 and (.baseline_rows | length) == 60 and
  (.events | length) == 4 and
  ([.tradeoffs[] | select(.breach_dominates or .stabilize_dominates)] | length) == 0
' "$evidence_dir/operation-lab-before.json" >/dev/null
jq -e '
  .schema == "RevenantOperationDomainLabV1" and
  ([.cases[] | select(.case == "selection_retry_different_seed" and
    .accepted == true and .disposition == "replayed")] | length) == 1 and
  ([.cases[] | select(.case == "capability_incomplete" and
    .accepted == false and .unchanged_on_error == true)] | length) == 1 and
  ([.cases[] | select(.case == "non_leader" and
    .accepted == false and .unchanged_on_error == true)] | length) == 1 and
  ([.cases[] | select(.case == "timeout_after_deadline" and
    .accepted == true)] | length) == 1
' "$evidence_dir/operation-domain-before.json" >/dev/null

username_prefix="m27x-$run_token-"
account_prefix="local:$username_prefix"
user_a="${username_prefix}a"
user_b="${username_prefix}b"
user_empty="${username_prefix}e"
user_old="${username_prefix}o"
user_v1="${username_prefix}v"
user_cap_a="${username_prefix}ca"
user_cap_b="${username_prefix}cb"
user_disconnect="${username_prefix}d"
user_timeout="${username_prefix}t"
character_a="local:$user_a:operator"
character_b="local:$user_b:operator"
assert_sql 0 preexisting-matrix-accounts "SELECT count(*) FROM accounts WHERE id LIKE '$account_prefix%';"

start_gateway 1 '0,1,0,2,0,1' p1
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
    reconcile_baseline "$username" "" 1 1 0 0 "$label"
  done
  label="prep-$account_code-5-mutate"
  run_plain_client "$username" mutate "$evidence_dir/$label-client.log"
  grep -q '^M26_MODULE_FLOW ' "$evidence_dir/$label-client.log"
  finish_session
  reconcile_baseline "$username" "" 1 1 2 1 "$label"
done

run_plain_client "$user_old" "" "$evidence_dir/baseline-v2-client.log"
finish_session
reconcile_baseline "$user_old" "" 1 1 0 0 baseline-v2

run_v1_client "$user_v1" "$evidence_dir/baseline-v1-probe-client.log"
grep -q '^M26_V1_PROBE ' "$evidence_dir/baseline-v1-probe-client.log"
finish_session
reconcile_baseline "$user_v1" "" 1 0 0 0 baseline-v1-probe

frozen_before="$(resolve_session revenant-frozen-v1)"
REVENANT_GAME_ADDR="$game_addr" timeout 15s "$frozen_bin" >"$evidence_dir/frozen-v1-client.log" 2>&1
grep -q 'frozen client V1 build 0.1.0-frozen' "$evidence_dir/frozen-v1-client.log"
finish_session
frozen_session="$(resolve_session revenant-frozen-v1)"
if [[ -z "$frozen_session" || "$frozen_session" == "$frozen_before" ]]; then
  echo "frozen V1 did not create a fresh compatibility session" >&2
  exit 1
fi
reconcile_frozen_v1 "$frozen_session" frozen-v1

execute_solo_route p1-breach-armor "$user_empty" breach overcharged_armor 0 pulse_rifle empty "" ""
execute_solo_route p1-breach-arc "$user_a" breach arc_surge 1 arc_sidearm force-ward force "m27-sa1-$run_token"
execute_solo_route p1-stabilize-shield "$user_a" stabilize shielded_channel 0 pulse_rifle force ward "m27-sa2-$run_token"
execute_solo_route p1-stabilize-residual "$user_a" stabilize residual_feedback 2 arc_sidearm ward force-ward "m27-sa3-$run_token"

run_route_probe "$user_disconnect" stabilize disconnect pulse_rifle "$evidence_dir/p1-disconnect-client.log"
disconnect_json="$(sed -n 's/^M27_ROUTE_DISCONNECT //p' "$evidence_dir/p1-disconnect-client.log")"
finish_session
reconcile_disconnect "$user_disconnect" p1-disconnect "$disconnect_json"

run_route_probe "$user_timeout" breach timeout arc_sidearm "$evidence_dir/p1-timeout-client.log"
timeout_json="$(sed -n 's/^M27_ROUTE_TIMEOUT //p' "$evidence_dir/p1-timeout-client.log")"
finish_session
reconcile_timeout "$user_timeout" p1-timeout "$timeout_json"

capture_metrics p1-after
assert_resource_bounds p1 "$p1_before_rss" "$p1_before_fds" "$p1_before_threads" "$p1_before_connections"
if [[ "$(grep -Ec 'session_command_failed|connection_failed|session_abort|seed queue was exhausted' "$gateway_log" || true)" -ne 0 ]]; then
  echo "p1 gateway logged an unexpected failure" >&2
  exit 1
fi
if [[ "$(grep -c '"event":"session_reset"' "$gateway_log" || true)" -ne 19 ]]; then
  echo "p1 gateway did not reset exactly 19 times" >&2
  exit 1
fi
stop_gateway

finalize_evidence() {
assert_sql 9 matrix-account-count "SELECT count(*) FROM accounts WHERE id LIKE '$account_prefix%';"
assert_sql 23 matrix-session-count "SELECT count(DISTINCT session_id) FROM replay_events WHERE account_id LIKE '$account_prefix%' AND event_type = 'player_joined';"
assert_sql 28 matrix-participant-joins "SELECT count(*) FROM replay_events WHERE account_id LIKE '$account_prefix%' AND event_type = 'player_joined';"
assert_sql 300 matrix-replay-event-count "SELECT count(*) FROM replay_events WHERE account_id LIKE '$account_prefix%';"
assert_sql 26 matrix-history-count "SELECT count(*) FROM activity_history WHERE account_id LIKE '$account_prefix%' AND activity_id = 'relay_awakening';"
assert_sql 24 matrix-fragment-total "SELECT COALESCE(sum(i.quantity), 0) FROM inventory i JOIN characters c ON c.id = i.character_id WHERE c.account_id LIKE '$account_prefix%' AND i.item_id = 'relay_core_fragment';"
assert_sql 2900 matrix-experience-total "SELECT COALESCE(sum(p.experience), 0) FROM progression p JOIN characters c ON c.id = p.character_id WHERE c.account_id LIKE '$account_prefix%';"
assert_sql 26 matrix-inventory-grants "SELECT count(*) FROM inventory_reward_grants g JOIN characters c ON c.id = g.character_id WHERE c.account_id LIKE '$account_prefix%';"
assert_sql 26 matrix-progression-grants "SELECT count(*) FROM progression_reward_grants g JOIN characters c ON c.id = g.character_id WHERE c.account_id LIKE '$account_prefix%';"
assert_sql 10 matrix-route-rows "SELECT count(*) FROM route_operations WHERE leader_account_id LIKE '$account_prefix%';"
assert_sql 14 matrix-route-participants "SELECT count(*) FROM route_operation_participants rp JOIN route_operations ro USING(session_id) WHERE ro.leader_account_id LIKE '$account_prefix%';"
assert_sql 10 matrix-route-selection-events "SELECT count(*) FROM replay_events WHERE account_id LIKE '$account_prefix%' AND event_type = 'route_selected';"
assert_sql 8 matrix-route-success-events "SELECT count(*) FROM replay_events WHERE account_id LIKE '$account_prefix%' AND event_type = 'route_operation_succeeded';"
assert_sql 1 matrix-route-failure-events "SELECT count(*) FROM replay_events WHERE account_id LIKE '$account_prefix%' AND event_type = 'route_operation_failed';"
assert_sql 1 matrix-route-incomplete-rows "SELECT count(*) FROM route_operations WHERE leader_account_id LIKE '$account_prefix%' AND terminal_outcome IS NULL;"
assert_sql 12 matrix-route-success-grants "SELECT COALESCE(sum(jsonb_array_length(payload::jsonb->'grants')), 0) FROM replay_events WHERE account_id LIKE '$account_prefix%' AND event_type = 'route_operation_succeeded';"
assert_sql 2 matrix-module-state-count "SELECT count(*) FROM module_states ms JOIN characters c ON c.id = ms.character_id WHERE c.account_id LIKE '$account_prefix%';"
assert_sql 4 matrix-owned-module-count "SELECT count(*) FROM inventory i JOIN characters c ON c.id = i.character_id WHERE c.account_id LIKE '$account_prefix%' AND i.item_id LIKE 'module_%';"
assert_sql 3 matrix-final-loadout-slots "SELECT count(*) FROM module_loadout_slots mls JOIN characters c ON c.id = mls.character_id WHERE c.account_id LIKE '$account_prefix%';"
assert_sql 4 matrix-combination-operations "SELECT count(*) FROM module_operations mo JOIN characters c ON c.id = mo.character_id WHERE c.account_id LIKE '$account_prefix%' AND mo.operation_kind = 'combine';"
assert_sql 11 matrix-loadout-operations "SELECT count(*) FROM module_operations mo JOIN characters c ON c.id = mo.character_id WHERE c.account_id LIKE '$account_prefix%' AND mo.operation_kind = 'loadout';"
assert_sql 11 matrix-revision-total "SELECT COALESCE(sum(ms.revision), 0) FROM module_states ms JOIN characters c ON c.id = ms.character_id WHERE c.account_id LIKE '$account_prefix%';"
assert_sql 28 matrix-snapshot-events "SELECT count(*) FROM replay_events WHERE account_id LIKE '$account_prefix%' AND event_type = 'module_state_snapshot';"
assert_sql 4 matrix-combination-events "SELECT count(*) FROM replay_events WHERE account_id LIKE '$account_prefix%' AND event_type = 'module_combined';"
assert_sql 11 matrix-loadout-events "SELECT count(*) FROM replay_events WHERE account_id LIKE '$account_prefix%' AND event_type = 'module_loadout_changed';"
assert_sql '9|7|module_force_matrix,module_ward_capacitor' matrix-final-account-a "SELECT i.quantity || '|' || ms.revision || '|' || string_agg(mls.module_item_id, ',' ORDER BY mls.slot_index) FROM characters c JOIN inventory i ON i.character_id = c.id AND i.item_id = 'relay_core_fragment' JOIN module_states ms ON ms.character_id = c.id JOIN module_loadout_slots mls ON mls.character_id = c.id WHERE c.id = '$character_a' GROUP BY i.quantity, ms.revision;"
assert_sql '7|4|module_force_matrix' matrix-final-account-b "SELECT i.quantity || '|' || ms.revision || '|' || string_agg(mls.module_item_id, ',' ORDER BY mls.slot_index) FROM characters c JOIN inventory i ON i.character_id = c.id AND i.item_id = 'relay_core_fragment' JOIN module_states ms ON ms.character_id = c.id JOIN module_loadout_slots mls ON mls.character_id = c.id WHERE c.id = '$character_b' GROUP BY i.quantity, ms.revision;"
assert_sql 0 matrix-route-integrity "WITH route_counts AS (SELECT ro.session_id, ro.terminal_outcome, ro.participant_count, count(*) FILTER (WHERE re.event_type = 'route_selected') selections, count(*) FILTER (WHERE re.event_type = 'route_operation_succeeded') successes, count(*) FILTER (WHERE re.event_type = 'route_operation_failed') failures, count(*) FILTER (WHERE re.event_type = 'activity_completed') completions, count(*) FILTER (WHERE re.event_type = 'loot_granted') loot, count(*) FILTER (WHERE re.event_type = 'progression_granted') progression FROM route_operations ro LEFT JOIN replay_events re USING(session_id) WHERE ro.leader_account_id LIKE '$account_prefix%' GROUP BY ro.session_id, ro.terminal_outcome, ro.participant_count) SELECT count(*) FROM route_counts WHERE selections <> 1 OR (terminal_outcome = 'succeeded' AND (successes <> 1 OR failures <> 0 OR completions <> 1 OR loot <> participant_count OR progression <> participant_count)) OR (terminal_outcome = 'failed_timeout' AND (successes <> 0 OR failures <> 1 OR completions <> 0 OR loot <> 0 OR progression <> 0)) OR (terminal_outcome IS NULL AND (successes <> 0 OR failures <> 0 OR completions <> 0 OR loot <> 0 OR progression <> 0));"
assert_sql 0 matrix-failure-injection-triggers "SELECT count(*) FROM pg_trigger WHERE NOT tgisinternal AND tgname ILIKE '%m27%';"
assert_sql 0 matrix-failure-injection-functions "SELECT count(*) FROM pg_proc WHERE proname ILIKE '%m27%';"

jq -s -e '
  length == 12 and
  ([.[] | select(.role == "primary")] | length) == 8 and
  ([.[] | select(.role == "secondary")] | length) == 4 and
  ([.[] | select(.participants == 1)] | length) == 4 and
  ([.[] | select(.participants == 2)] | length) == 8 and
  ([.[] | select(.weapon == "pulse_rifle")] | length) == 6 and
  ([.[] | select(.weapon == "arc_sidearm")] | length) == 6 and
  ([.[] | select(.route_id == "breach")] | length) == 6 and
  ([.[] | select(.route_id == "stabilize")] | length) == 6 and
  ([.[] | select(.event_id == "overcharged_armor")] | length) == 3 and
  ([.[] | select(.event_id == "arc_surge")] | length) == 3 and
  ([.[] | select(.event_id == "shielded_channel")] | length) == 3 and
  ([.[] | select(.event_id == "residual_feedback")] | length) == 3 and
  ([.[] | select(.module_build == "empty")] | length) == 2 and
  ([.[] | select(.module_build == "force")] | length) == 2 and
  ([.[] | select(.module_build == "ward")] | length) == 3 and
  ([.[] | select(.module_build == "force-ward")] | length) == 5 and
  ([.[] | select(.role == "primary" and .retry_replayed and .conflict_rejected)] | length) == 8 and
  ([.[] | select(.role == "secondary" and .non_leader_rejected)] | length) == 4
' "$evidence_dir/client-matrix.jsonl" >/dev/null
jq -s -e '
  length == 23 and
  ([.[] | select(.route_replay_legacy == true)] | length) == 13 and
  ([.[] | select(.route_replay_legacy == false)] | length) == 10 and
  ([.[] | select(.completed == true)] | length) == 21 and
  ([.[] | select(.route_terminal_outcome == "succeeded")] | length) == 8 and
  ([.[] | select(.route_terminal_outcome == "failed_timeout")] | length) == 1 and
  ([.[] | select(.route_replay_legacy == false and .route_terminal_outcome == null)] | length) == 1
' "$evidence_dir/session-summaries.jsonl" >/dev/null

jq -s '
  group_by([.participants, .weapon, .module_build, .route_id, .event_id]) |
  map({
    participants: .[0].participants,
    weapon: .[0].weapon,
    module_build: .[0].module_build,
    route_id: .[0].route_id,
    event_id: .[0].event_id,
    clients: length,
    warden_health: (map(.warden.target_health) | unique),
    warden_hits: (map(.warden.accepted_hits) | unique),
    counter_damage: (map(.warden_counter_damage) | unique),
    elapsed_ms: (map(.elapsed_ms) | unique)
  })
' "$evidence_dir/client-matrix.jsonl" >"$evidence_dir/coverage-summary.json"

"$operation_lab_bin" --json >"$evidence_dir/operation-lab-after.json"
"$operation_lab_bin" --domain-json >"$evidence_dir/operation-domain-after.json"
cmp -s "$evidence_dir/operation-lab-before.json" "$evidence_dir/operation-lab-after.json"
cmp -s "$evidence_dir/operation-domain-before.json" "$evidence_dir/operation-domain-after.json"
sha256sum "$evidence_dir/operation-lab-before.json" "$evidence_dir/operation-domain-before.json" >"$evidence_dir/operation-hashes.txt"

main_hash="$(sha256sum "$repo_root/archive/clients/v1/src/main.rs" | cut -d' ' -f1)"
cargo_hash="$(sha256sum "$repo_root/archive/clients/v1/Cargo.toml" | cut -d' ' -f1)"
[[ "$main_hash" == 4f481e9fc5d22a5ab6d8f2d0a40e2d05dc9aaf92099debdd9dedf59c26f31f72 ]]
[[ "$cargo_hash" == c951c5fe88daa2dd9fb91a4da98ca316fd3923e0bff5332d748db44bce367322 ]]
printf 'src/main.rs  %s\nCargo.toml  %s\n' "$main_hash" "$cargo_hash" >"$evidence_dir/frozen-v1-hashes.txt"

finished_epoch="$(date +%s)"
duration_seconds=$((finished_epoch - started_epoch))
printf 'run_token=%s\nprefixed_accounts=9\nprefixed_sessions=23\nparticipant_joins=28\ncompleted_histories=26\nrouted_success_sessions=8\nrouted_success_clients=12\nrouted_failure_sessions=1\nrouted_incomplete_sessions=1\ncompatibility_sessions=14\nduration_seconds=%s\n' "$run_token" "$duration_seconds" >"$evidence_dir/summary.txt"

(
  cd "$evidence_dir"
  find . -type f ! -name SHA256SUMS -print0 | sort -z | xargs -0 sha256sum >SHA256SUMS
  sha256sum --check --quiet SHA256SUMS
)

printf 'M27 runtime matrix passed: %s\n' "$evidence_dir"
}

start_gateway 2 '0,1,0,2' p2
capture_metrics p2-before
p2_before_rss="$METRIC_RSS"
p2_before_fds="$METRIC_FDS"
p2_before_threads="$METRIC_THREADS"
p2_before_connections="$METRIC_CONNECTIONS"

execute_capability_mismatch p2-capability-mismatch "$user_cap_a" "$user_cap_b"
execute_two_active_route p2-breach-armor breach overcharged_armor 0 pulse_rifle "$user_empty" empty "" "" "$user_b" force-ward ward "m27-p2b1-$run_token" 1
execute_two_active_route p2-breach-arc breach arc_surge 1 arc_sidearm "$user_a" force-ward force "m27-p2a2-$run_token" "$user_b" ward force-ward "m27-p2b2-$run_token" 2
execute_two_active_route p2-stabilize-shield stabilize shielded_channel 0 pulse_rifle "$user_a" force ward "m27-p2a3-$run_token" "$user_b" force-ward "" "" 1
execute_two_active_route p2-stabilize-residual stabilize residual_feedback 2 arc_sidearm "$user_a" ward force-ward "m27-p2a4-$run_token" "$user_b" force-ward force "m27-p2b4-$run_token" 2

capture_metrics p2-after
assert_resource_bounds p2 "$p2_before_rss" "$p2_before_fds" "$p2_before_threads" "$p2_before_connections"
if [[ "$(grep -Ec 'session_command_failed|connection_failed|session_abort|seed queue was exhausted' "$gateway_log" || true)" -ne 0 ]]; then
  echo "p2 gateway logged an unexpected failure" >&2
  exit 1
fi
if [[ "$(grep -c '"event":"session_reset"' "$gateway_log" || true)" -ne 5 ]]; then
  echo "p2 gateway did not reset exactly five times" >&2
  exit 1
fi
stop_gateway
finalize_evidence
