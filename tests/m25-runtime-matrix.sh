#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$repo_root/scripts/database-url.sh"
gateway_bin="${M25_GATEWAY_BIN:-$repo_root/target/debug/revenant-gateway}"
bot_bin="${M25_BOT_BIN:-$repo_root/target/debug/revenant-bot}"
cli_bin="${M25_CLI_BIN:-$repo_root/target/debug/revenant}"
health_addr="${M25_MATRIX_HEALTH_ADDR:-127.0.0.1:18084}"
game_addr="${M25_MATRIX_GAME_ADDR:-127.0.0.1:17004}"
inspector_origin="${REVENANT_INSPECTOR_ORIGIN:-http://127.0.0.1:4173}"
database_url="$(revenant_database_url)"
evidence_root="${M25_MATRIX_EVIDENCE_ROOT:-$repo_root/../revenant-local-evidence}"
run_token="${M25_MATRIX_RUN_TOKEN:-$(date -u +%H%M%S)-$$}"
round_trip_cases=(0 75 150 0 0)
weapons=(pulse_rifle arc_sidearm)

if [[ ! "$run_token" =~ ^[a-zA-Z0-9-]{1,16}$ ]]; then
  echo "M25_MATRIX_RUN_TOKEN must contain 1-16 ASCII letters, digits, or hyphens" >&2
  exit 1
fi
for executable in "$gateway_bin" "$bot_bin" "$cli_bin"; do
  if [[ ! -x "$executable" ]]; then
    echo "M25 runtime matrix executable is unavailable: $executable" >&2
    exit 1
  fi
done
for command in curl docker jq timeout; do
  if ! command -v "$command" >/dev/null; then
    echo "M25 runtime matrix command is unavailable: $command" >&2
    exit 1
  fi
done

mkdir -p "$evidence_root"
evidence_dir="$evidence_root/m25-runtime-$run_token"
if [[ -e "$evidence_dir" ]]; then
  echo "M25 runtime matrix evidence target already exists: $evidence_dir" >&2
  exit 1
fi
mkdir "$evidence_dir"

gateway_pid=""
primary_pid=""
secondary_pid=""
gateway_log=""

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
  for _ in {1..240}; do
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
  REVENANT_BIND_ADDR="$health_addr" \
  REVENANT_GAME_ADDR="$game_addr" \
  REVENANT_EXPECTED_PLAYERS="$expected_players" \
  REVENANT_DATABASE_MODE=existing \
  DATABASE_URL="$database_url" \
    "$gateway_bin" >"$gateway_log" 2>&1 &
  gateway_pid=$!
  if ! wait_for_gateway; then
    cat "$gateway_log" >&2
    echo "M25 runtime matrix gateway did not become healthy for $expected_players players" >&2
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

run_client() {
  local username="$1"
  local role="$2"
  local weapon="$3"
  local rtt_ms="$4"
  local expected_players="$5"
  local output="$6"
  REVENANT_GAME_ADDR="$game_addr" \
  REVENANT_BOT_USERNAME="$username" \
  REVENANT_BOT_ROLE="$role" \
  REVENANT_BOT_WEAPON="$weapon" \
  REVENANT_BOT_ROUND_TRIP_MS="$rtt_ms" \
  REVENANT_EXPECTED_PLAYERS="$expected_players" \
    timeout 45s "$bot_bin" >"$output" 2>&1
}

assert_client_json() {
  local json="$1"
  local weapon="$2"
  local rtt_ms="$3"
  local participants="$4"
  local expected_drone_hits="$5"
  local expected_warden_hits="$6"
  jq -e \
    --arg weapon "$weapon" \
    --argjson rtt "$rtt_ms" \
    --argjson participants "$participants" \
    --argjson drone_hits "$expected_drone_hits" \
    --argjson warden_hits "$expected_warden_hits" '
      .schema == "RevenantM25RuntimeMatrixV1" and
      .weapon == $weapon and
      .simulated_rtt_ms == $rtt and
      .participants == $participants and
      .attack_interval_ms == (if $weapon == "pulse_rifle" then 250 else 150 end) and
      .drone.accepted_hits == $drone_hits and
      .warden.accepted_hits == $warden_hits and
      .drone.remaining_health[-1] == 0 and
      .warden.remaining_health[-1] == 0 and
      .drone.hostile_damage == [10] and
      .warden.hostile_damage == [15, 15] and
      (.drone.hits_by_actor | length) == $participants and
      (.warden.hits_by_actor | length) == $participants and
      .loot_grants == 1 and
      .progression_grants == 1 and
      .experience_granted == 100 and
      .completed == true
    ' <<<"$json" >/dev/null
}

reconcile_session() {
  local username="$1"
  local secondary_username="$2"
  local participants="$3"
  local weapon="$4"
  local rtt_ms="$5"
  local run_index="$6"
  local label="$7"
  local session_id
  session_id="$(psql_value "SELECT session_id FROM replay_events WHERE account_id = 'local:$username' AND event_type = 'player_joined' ORDER BY id DESC LIMIT 1;")"
  if [[ -z "$session_id" || ! "$session_id" =~ ^session-[0-9]+$ ]]; then
    echo "$label did not resolve a valid replay session" >&2
    exit 1
  fi
  if [[ -n "$secondary_username" ]]; then
    local secondary_session
    secondary_session="$(psql_value "SELECT session_id FROM replay_events WHERE account_id = 'local:$secondary_username' AND event_type = 'player_joined' ORDER BY id DESC LIMIT 1;")"
    if [[ "$secondary_session" != "$session_id" ]]; then
      echo "$label active clients resolved different replay sessions" >&2
      exit 1
    fi
  fi

  local replay_file="$evidence_dir/$label-replay.log"
  DATABASE_URL="$database_url" "$cli_bin" replay --latest "local:$username" >"$replay_file"
  grep -q "^Replay session $session_id$" "$replay_file"
  grep -q "State: activity=relay_awakening enemies_spawned=1 enemies_defeated=2 boss_spawned=true" "$replay_file"
  grep -q "loot_grants=$participants progression_grants=$participants equipment_changes=$participants completed=true" "$replay_file"

  local summary_json
  local events_json
  summary_json="$(curl --fail --silent --header "Origin: $inspector_origin" "http://$health_addr/api/inspector/sessions/$session_id/summary")"
  events_json="$(curl --fail --silent --header "Origin: $inspector_origin" "http://$health_addr/api/inspector/sessions/$session_id/events")"
  jq -e --argjson participants "$participants" '
    .summary.completed == true and
    .summary.participant_count == $participants and
    .summary.enemy_spawn_count == 1 and
    .summary.enemy_defeat_count == 2 and
    .summary.boss_spawned == true and
    .summary.equipment_change_count == $participants and
    .summary.loot_grant_count == $participants and
    .summary.progression_grant_count == $participants and
    .summary.join_to_start_ms != null and
    .summary.activity_duration_ms != null
  ' <<<"$summary_json" >/dev/null
  jq -e --argjson participants "$participants" '
    ([.events[] | select(.event_type == "player_joined")] | length) == $participants and
    ([.events[] | select(.event_type == "activity_completed")] | length) == 1 and
    ([.events[] | select(.event_type == "enemy_died")] | length) == 2 and
    ([.events[] | select(.event_type == "boss_spawned")] | length) == 1 and
    ([.events[] | select(.event_type == "equipment_changed")] | length) == $participants and
    ([.events[] | select(.event_type == "loot_granted")] | length) == $participants and
    ([.events[] | select(.event_type == "progression_granted")] | length) == $participants
  ' <<<"$events_json" >/dev/null
  jq -c \
    --arg weapon "$weapon" \
    --argjson rtt "$rtt_ms" \
    --argjson run_index "$run_index" '
      .summary + {weapon: $weapon, simulated_rtt_ms: $rtt, run_index: $run_index}
    ' <<<"$summary_json" >>"$evidence_dir/session-summaries.jsonl"
}

printf 'phase\trss_kib\tdescriptors\tthreads\tpostgres_connections\n' \
  >"$evidence_dir/resource-metrics.tsv"
printf 'assertion\texpected\tactual\n' >"$evidence_dir/database-assertions.tsv"
: >"$evidence_dir/client-matrix.jsonl"
: >"$evidence_dir/session-summaries.jsonl"
started_epoch="$(date +%s)"
account_prefix="local:m25m-$run_token-"
assert_sql 0 preexisting-matrix-accounts \
  "SELECT count(*) FROM accounts WHERE id LIKE '$account_prefix%';"

for participants in 1 2; do
  start_gateway "$participants"
  capture_metrics "p$participants-before"
  before_rss="$METRIC_RSS"
  before_fds="$METRIC_FDS"
  before_threads="$METRIC_THREADS"
  before_connections="$METRIC_CONNECTIONS"
  completed_runs=0

  for weapon in "${weapons[@]}"; do
    weapon_code="r"
    expected_drone_hits=4
    expected_warden_hits=6
    if [[ "$weapon" == "arc_sidearm" ]]; then
      weapon_code="s"
      expected_drone_hits=6
      expected_warden_hits=10
    fi
    if (( participants == 2 )); then
      if [[ "$weapon" == "pulse_rifle" ]]; then
        expected_drone_hits=6
        expected_warden_hits=9
      else
        expected_drone_hits=9
        expected_warden_hits=15
      fi
    fi

    for case_index in "${!round_trip_cases[@]}"; do
      rtt_ms="${round_trip_cases[$case_index]}"
      run_index=$((case_index + 1))
      label="p${participants}-${weapon_code}-${run_index}-rtt${rtt_ms}"
      primary_username="m25m-$run_token-${participants}${weapon_code}${run_index}a"
      secondary_username=""
      primary_output="$evidence_dir/$label-primary.log"
      secondary_output="$evidence_dir/$label-secondary.log"

      if (( participants == 1 )); then
        run_client "$primary_username" matrix-primary "$weapon" "$rtt_ms" \
          "$participants" "$primary_output"
      else
        secondary_username="m25m-$run_token-${participants}${weapon_code}${run_index}b"
        run_client "$primary_username" matrix-primary "$weapon" "$rtt_ms" \
          "$participants" "$primary_output" &
        primary_pid=$!
        sleep 0.1
        run_client "$secondary_username" matrix-secondary "$weapon" "$rtt_ms" \
          "$participants" "$secondary_output" &
        secondary_pid=$!
        wait "$primary_pid"
        primary_pid=""
        wait "$secondary_pid"
        secondary_pid=""
      fi

      primary_json="$(sed -n 's/^M25_MATRIX //p' "$primary_output")"
      if [[ "$(printf '%s\n' "$primary_json" | wc -l)" -ne 1 ]]; then
        echo "$label did not emit exactly one primary matrix record" >&2
        exit 1
      fi
      assert_client_json "$primary_json" "$weapon" "$rtt_ms" "$participants" \
        "$expected_drone_hits" "$expected_warden_hits"
      jq -c --argjson run_index "$run_index" '. + {run_index: $run_index}' \
        <<<"$primary_json" >>"$evidence_dir/client-matrix.jsonl"

      if (( participants == 2 )); then
        secondary_json="$(sed -n 's/^M25_MATRIX //p' "$secondary_output")"
        if [[ "$(printf '%s\n' "$secondary_json" | wc -l)" -ne 1 ]]; then
          echo "$label did not emit exactly one secondary matrix record" >&2
          exit 1
        fi
        assert_client_json "$secondary_json" "$weapon" "$rtt_ms" "$participants" \
          "$expected_drone_hits" "$expected_warden_hits"
        primary_shared="$(jq -S 'del(.account_id, .role)' <<<"$primary_json")"
        secondary_shared="$(jq -S 'del(.account_id, .role)' <<<"$secondary_json")"
        if [[ "$primary_shared" != "$secondary_shared" ]]; then
          echo "$label active clients projected different combat state" >&2
          exit 1
        fi
      fi

      completed_runs=$((completed_runs + 1))
      wait_for_resets "$completed_runs"
      reconcile_session "$primary_username" "$secondary_username" "$participants" \
        "$weapon" "$rtt_ms" "$run_index" "$label"
      printf 'M25 runtime %s completed\n' "$label"
    done
  done

  sleep 0.2
  capture_metrics "p$participants-after"
  assert_resource_bounds "p$participants" "$before_rss" "$before_fds" \
    "$before_threads" "$before_connections"
  if [[ "$(grep -Ec 'session_command_failed|connection_failed|session_abort' "$gateway_log" || true)" -ne 0 ]]; then
    echo "p$participants gateway logged a command, connection, or abort failure" >&2
    exit 1
  fi
  if [[ "$(grep -c '"event":"session_reset"' "$gateway_log" || true)" -ne 10 ]]; then
    echo "p$participants gateway did not reset exactly ten times" >&2
    exit 1
  fi
  stop_gateway
done

assert_sql 30 matrix-account-count \
  "SELECT count(*) FROM accounts WHERE id LIKE '$account_prefix%';"
assert_sql 30 matrix-history-count \
  "SELECT count(*) FROM activity_history WHERE account_id LIKE '$account_prefix%' AND activity_id = 'relay_awakening';"
assert_sql 30 matrix-fragment-total \
  "SELECT COALESCE(sum(i.quantity), 0) FROM inventory i JOIN characters c ON c.id = i.character_id WHERE c.account_id LIKE '$account_prefix%' AND i.item_id = 'relay_core_fragment';"
assert_sql 3000 matrix-experience-total \
  "SELECT COALESCE(sum(p.experience), 0) FROM progression p JOIN characters c ON c.id = p.character_id WHERE c.account_id LIKE '$account_prefix%';"
assert_sql 30 matrix-inventory-grants \
  "SELECT count(*) FROM inventory_reward_grants g JOIN characters c ON c.id = g.character_id WHERE c.account_id LIKE '$account_prefix%';"
assert_sql 30 matrix-progression-grants \
  "SELECT count(*) FROM progression_reward_grants g JOIN characters c ON c.id = g.character_id WHERE c.account_id LIKE '$account_prefix%';"
assert_sql 20 matrix-session-count \
  "SELECT count(DISTINCT session_id) FROM replay_events WHERE account_id LIKE '$account_prefix%' AND event_type = 'player_joined';"
assert_sql 0 matrix-session-integrity \
  "WITH selected AS (SELECT DISTINCT session_id FROM replay_events WHERE account_id LIKE '$account_prefix%' AND event_type = 'player_joined'), counts AS (SELECT r.session_id, count(*) FILTER (WHERE event_type = 'player_joined') joins, count(*) FILTER (WHERE event_type = 'activity_completed') completions, count(*) FILTER (WHERE event_type = 'loot_granted') loot, count(*) FILTER (WHERE event_type = 'progression_granted') progression, count(*) FILTER (WHERE event_type = 'equipment_changed') equipment, count(*) FILTER (WHERE event_type = 'enemy_died') deaths, count(*) FILTER (WHERE event_type = 'boss_spawned') bosses FROM replay_events r JOIN selected s ON s.session_id = r.session_id GROUP BY r.session_id) SELECT count(*) FROM counts WHERE joins NOT IN (1, 2) OR completions <> 1 OR loot <> joins OR progression <> joins OR equipment <> joins OR deaths <> 2 OR bosses <> 1;"

jq -s -e '
  length == 20 and
  ([.[] | select(.weapon == "pulse_rifle")] | length) == 10 and
  ([.[] | select(.weapon == "arc_sidearm")] | length) == 10 and
  ([.[] | select(.participants == 1)] | length) == 10 and
  ([.[] | select(.participants == 2)] | length) == 10 and
  ([.[] | if .participants == 1 then
    (.drone.ttk_ms >= 600 and .drone.ttk_ms <= 1200 and
     .warden.ttk_ms >= 1200 and .warden.ttk_ms <= 2600)
  else true end] | all)
' "$evidence_dir/client-matrix.jsonl" >/dev/null

jq -s '
  group_by([.weapon, .participants, .simulated_rtt_ms]) |
  map({
    weapon: .[0].weapon,
    participants: .[0].participants,
    simulated_rtt_ms: .[0].simulated_rtt_ms,
    runs: length,
    drone_ttk_ms: {
      minimum: (map(.drone.ttk_ms) | min),
      average: ((map(.drone.ttk_ms) | add) / length | floor),
      maximum: (map(.drone.ttk_ms) | max)
    },
    warden_ttk_ms: {
      minimum: (map(.warden.ttk_ms) | min),
      average: ((map(.warden.ttk_ms) | add) / length | floor),
      maximum: (map(.warden.ttk_ms) | max)
    }
  })
' "$evidence_dir/client-matrix.jsonl" >"$evidence_dir/ttk-summary.json"

for weapon in "${weapons[@]}"; do
  for rtt_ms in 0 75 150; do
    solo_drone="$(jq -r --arg weapon "$weapon" --argjson rtt "$rtt_ms" '.[] | select(.weapon == $weapon and .participants == 1 and .simulated_rtt_ms == $rtt) | .drone_ttk_ms.average' "$evidence_dir/ttk-summary.json")"
    multi_drone="$(jq -r --arg weapon "$weapon" --argjson rtt "$rtt_ms" '.[] | select(.weapon == $weapon and .participants == 2 and .simulated_rtt_ms == $rtt) | .drone_ttk_ms.average' "$evidence_dir/ttk-summary.json")"
    solo_warden="$(jq -r --arg weapon "$weapon" --argjson rtt "$rtt_ms" '.[] | select(.weapon == $weapon and .participants == 1 and .simulated_rtt_ms == $rtt) | .warden_ttk_ms.average' "$evidence_dir/ttk-summary.json")"
    multi_warden="$(jq -r --arg weapon "$weapon" --argjson rtt "$rtt_ms" '.[] | select(.weapon == $weapon and .participants == 2 and .simulated_rtt_ms == $rtt) | .warden_ttk_ms.average' "$evidence_dir/ttk-summary.json")"
    if (( multi_drone * 100 < solo_drone * 45 || multi_warden * 100 < solo_warden * 45 )); then
      echo "$weapon RTT $rtt_ms violated the 45% multiplayer TTK floor" >&2
      exit 1
    fi
  done
done

finished_epoch="$(date +%s)"
duration_seconds=$((finished_epoch - started_epoch))
printf 'run_token=%s\nsessions=20\naccounts=30\npulse_rifle_sessions=10\narc_sidearm_sessions=10\nsolo_sessions=10\ntwo_active_sessions=10\nsimulated_rtt_cases_ms=0,75,150\nduration_seconds=%s\n' \
  "$run_token" "$duration_seconds" >"$evidence_dir/summary.txt"

(
  cd "$evidence_dir"
  find . -type f ! -name SHA256SUMS -print0 | sort -z | xargs -0 sha256sum >SHA256SUMS
  sha256sum --check --quiet SHA256SUMS
)

echo "M25 runtime matrix passed: 20 sessions, 30 active clients, both weapons, RTT 0/75/150 ms"
echo "M25_MATRIX_EVIDENCE=$evidence_dir"
