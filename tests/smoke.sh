#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
bind_addr="127.0.0.1:18080"
game_addr="127.0.0.1:17000"
inspector_origin="${REVENANT_INSPECTOR_ORIGIN:-http://127.0.0.1:4173}"
log_file="$(mktemp)"
reconstruction_log="$(mktemp)"
godot_data_dir="$(mktemp -d)"
godot_project_dir=""
bot_bin="${CARGO_TARGET_DIR:-$repo_root/target}/debug/revenant-bot"

cleanup() {
  if [[ -n "${observer_pid:-}" ]]; then
    kill "$observer_pid" 2>/dev/null || true
    wait "$observer_pid" 2>/dev/null || true
  fi
  if [[ -n "${server_pid:-}" ]]; then
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  if [[ -n "${reconstruction_pid:-}" ]]; then
    kill "$reconstruction_pid" 2>/dev/null || true
    wait "$reconstruction_pid" 2>/dev/null || true
  fi
  rm -f "$log_file"
  rm -f "$reconstruction_log"
  rm -rf "$godot_data_dir"
  if [[ -n "$godot_project_dir" ]]; then
    rm -rf "$godot_project_dir"
  fi
}
trap cleanup EXIT

cd "$repo_root"

start_gateway() {
  REVENANT_BIND_ADDR="$bind_addr" REVENANT_GAME_ADDR="$game_addr" \
    REVENANT_EXPECTED_PLAYERS="${expected_players:-1}" \
    REVENANT_DATABASE_MODE=existing \
    cargo run --quiet -p revenant-gateway >"$log_file" 2>&1 &
  server_pid=$!
}

wait_for_gateway() {
  for _ in {1..180}; do
    if response="$(curl --fail --silent "http://$bind_addr/health")"; then
      [[ "$response" == *'"status":"ok"'* ]]
      return 0
    fi
    sleep 0.2
  done
  return 1
}

wait_for_session_resets() {
  local expected="$1"
  for _ in {1..100}; do
    if [[ "$(grep -c '"event":"session_reset"' "$log_file" || true)" -ge "$expected" ]]; then
      return 0
    fi
    sleep 0.1
  done
  return 1
}

assert_pressure_counts() {
  local expected_sessions="$1"
  local total
  local drone_hits
  local warden_hits
  local lethal_hits
  total="$(grep -c '"event":"enemy_attack_applied"' "$log_file" || true)"
  drone_hits="$(grep -c '"event":"enemy_attack_applied".*"damage":10' "$log_file" || true)"
  warden_hits="$(grep -c '"event":"enemy_attack_applied".*"damage":15' "$log_file" || true)"
  lethal_hits="$(grep -c '"event":"enemy_attack_applied".*"killed":true' "$log_file" || true)"
  [[ "$total" -eq $((expected_sessions * 3)) ]]
  [[ "$drone_hits" -eq "$expected_sessions" ]]
  [[ "$warden_hits" -eq $((expected_sessions * 2)) ]]
  [[ "$lethal_hits" -eq 0 ]]
}

expected_players=2
cargo build --quiet -p revenant-bot
start_gateway

if wait_for_gateway; then
    REVENANT_GAME_ADDR="$game_addr" \
      REVENANT_EXPECTED_PLAYERS=2 \
      REVENANT_BOT_USERNAME="revenant-observer" \
      REVENANT_BOT_ROLE="observer" \
      "$bot_bin" &
    observer_pid=$!
    sleep 0.2
    REVENANT_GAME_ADDR="$game_addr" \
      REVENANT_EXPECTED_PLAYERS=2 \
      REVENANT_BOT_USERNAME="revenant-driver" \
      REVENANT_BOT_ROLE="driver" \
      "$bot_bin"
    wait "$observer_pid"
    observer_pid=""
    assert_pressure_counts 1

    if [[ -n "${GODOT_BIN:-}" ]]; then
      godot_project_dir="$(mktemp -d)"
      (
        cd client/game
        tar --exclude='./.godot' -cf - .
      ) | (
        cd "$godot_project_dir"
        tar -xf -
      )
      XDG_DATA_HOME="$godot_data_dir/data" \
      XDG_CONFIG_HOME="$godot_data_dir/config" \
      XDG_CACHE_HOME="$godot_data_dir/cache" \
        "$GODOT_BIN" --headless --editor --path "$godot_project_dir" --quit

      kill "$server_pid"
      wait "$server_pid" 2>/dev/null || true
      server_pid=""
      expected_players=1
      start_gateway
      if ! wait_for_gateway; then
        cat "$log_file"
        echo "single-player gateway did not become ready for Godot" >&2
        exit 1
      fi
      XDG_DATA_HOME="$godot_data_dir/data" \
      XDG_CONFIG_HOME="$godot_data_dir/config" \
      XDG_CACHE_HOME="$godot_data_dir/cache" \
      REVENANT_GAME_HOST="127.0.0.1" \
      REVENANT_GAME_PORT="17000" \
      REVENANT_EXIT_AFTER_FLOW="1" \
        timeout 15s "$GODOT_BIN" --headless --path "$godot_project_dir"

      wait_for_session_resets 1
      reusable_session_output="$(XDG_DATA_HOME="$godot_data_dir/data" \
        XDG_CONFIG_HOME="$godot_data_dir/config" \
        XDG_CACHE_HOME="$godot_data_dir/cache" \
        REVENANT_GAME_HOST="127.0.0.1" \
        REVENANT_GAME_PORT="17000" \
        REVENANT_EXIT_AFTER_FLOW="1" \
          timeout 15s "$GODOT_BIN" --headless --path "$godot_project_dir")"
      echo "$reusable_session_output"
      [[ "$reusable_session_output" == *"activity relay_awakening completed"* ]]

      wait_for_session_resets 2
      manual_output="$(XDG_DATA_HOME="$godot_data_dir/data" \
        XDG_CONFIG_HOME="$godot_data_dir/config" \
        XDG_CACHE_HOME="$godot_data_dir/cache" \
        REVENANT_GAME_HOST="127.0.0.1" \
        REVENANT_GAME_PORT="17000" \
        REVENANT_VALIDATE_MANUAL_FLOW="1" \
          timeout 20s "$GODOT_BIN" --headless --path "$godot_project_dir")"
      echo "$manual_output"
      [[ "$manual_output" == *"M17 manual controls completed relay_awakening without user input"* ]]

      wait_for_session_resets 3
      keyboard_output="$(XDG_DATA_HOME="$godot_data_dir/data" \
        XDG_CONFIG_HOME="$godot_data_dir/config" \
        XDG_CACHE_HOME="$godot_data_dir/cache" \
        REVENANT_GAME_HOST="127.0.0.1" \
        REVENANT_GAME_PORT="17000" \
        REVENANT_GAME_USERNAME="revenant-keyboard-driver" \
        REVENANT_VALIDATE_KEYBOARD_FLOW="1" \
          timeout 20s "$GODOT_BIN" --headless --path "$godot_project_dir")"
      echo "$keyboard_output"
      [[ "$keyboard_output" == *"M24 keyboard-only flow validated"* ]]

      slice_output="$(XDG_DATA_HOME="$godot_data_dir/data" \
        XDG_CONFIG_HOME="$godot_data_dir/config" \
        XDG_CACHE_HOME="$godot_data_dir/cache" \
        REVENANT_VALIDATE_SLICE="1" \
          timeout 10s "$GODOT_BIN" --headless --path "$godot_project_dir")"
      echo "$slice_output"
      [[ "$slice_output" == *"M17 playable slice validated"* ]]
      [[ "$slice_output" == *"M18 inventory HUD validated"* ]]
      [[ "$slice_output" == *"M19 progression HUD validated"* ]]
      [[ "$slice_output" == *"M20 loadout HUD validated"* ]]
      [[ "$slice_output" == *"M21 Operator validated"* ]]
      [[ "$slice_output" == *"M21 relay-hub environment validated"* ]]
      [[ "$slice_output" == *"M21 enemies validated"* ]]
      [[ "$slice_output" == *"M21 combat VFX validated"* ]]
      [[ "$slice_output" == *"M21 Operator HUD validated"* ]]
      [[ "$slice_output" == *"M21 presentation polish validated"* ]]
      [[ "$slice_output" == *"M21 presentation captures validated"* ]]
      [[ "$slice_output" == *"M22 entry shell validated"* ]]
      [[ "$slice_output" == *"M22 settings validated"* ]]
      [[ "$slice_output" == *"M22 onboarding validated"* ]]
      [[ "$slice_output" == *"M22 audio foundation validated"* ]]
      [[ "$slice_output" == *"M22 combat audio validated"* ]]
      [[ "$slice_output" == *"M22 integration evidence validated"* ]]
      [[ "$slice_output" == *"M23 MessagePack boundary validated"* ]]
      [[ "$slice_output" == *"M23 framed transport validated"* ]]
      [[ "$slice_output" == *"M23 session controller validated"* ]]
      [[ "$slice_output" == *"M23 authoritative state projection validated"* ]]
      [[ "$slice_output" == *"M23 input and HUD coordination validated"* ]]
      [[ "$slice_output" == *"M24 local observation validated"* ]]
      [[ "$slice_output" == *"M24 display and first-contact validated"* ]]
      [[ "$slice_output" == *"M25 authoritative combat cadence validated"* ]]
      [[ "$slice_output" == *"M25 honest combat presentation validated"* ]]
      [[ "$slice_output" == *"M26 honest module workshop validated"* ]]
      [[ "$slice_output" == *"M27 honest route console validated"* ]]

      wait_for_session_resets 4
      assert_pressure_counts 4
    fi

    kill "$server_pid"
    wait "$server_pid" 2>/dev/null || true
    server_pid=""
    start_gateway
    if ! wait_for_gateway; then
      cat "$log_file"
      echo "gateway did not become ready after restart" >&2
      exit 1
    fi
    REVENANT_EXPECT_ACCOUNT="local:revenant-driver" \
      cargo run --quiet -p revenant-persistence --bin persistence-check

    replay_output="$(cargo run --quiet -p revenant-cli -- replay --latest local:revenant-driver)"
    echo "$replay_output"
    for expected in \
      "player joined" \
      "activity started" \
      "enemy spawned" \
      "enemy died" \
      "boss spawned" \
      "activity completed" \
      "loot granted: relay_core_fragment x1" \
      "loot_grants=2" \
      "progression granted: +100 XP" \
      "progression_grants=2" \
      "weapon equipped: arc_sidearm" \
      "equipment_changes=1" \
      "completed=true"; do
      if [[ "$replay_output" != *"$expected"* ]]; then
        echo "replay timeline is missing: $expected" >&2
        exit 1
      fi
    done

    sessions_json="$(curl --fail --silent --header "Origin: $inspector_origin" "http://$bind_addr/api/inspector/sessions")"
    [[ "$sessions_json" == *'"session_id"'* ]]
    [[ "$sessions_json" == *'"participant_count":2'* ]]
    session_id="$(printf '%s' "$replay_output" | sed -n '1s/^Replay session //p')"
    events_json="$(curl --fail --silent --header "Origin: $inspector_origin" "http://$bind_addr/api/inspector/sessions/$session_id/events")"
    [[ "$events_json" == *'"event_type":"player_joined"'* ]]
    [[ "$events_json" == *'"event_type":"activity_completed"'* ]]
    summary_json="$(curl --fail --silent --header "Origin: $inspector_origin" "http://$bind_addr/api/inspector/sessions/$session_id/summary")"
    [[ "$summary_json" == *'"completed":true'* ]]
    [[ "$summary_json" == *'"participant_count":2'* ]]
    [[ "$summary_json" == *'"enemy_spawn_count":1'* ]]
    [[ "$summary_json" == *'"enemy_defeat_count":2'* ]]
    [[ "$summary_json" == *'"boss_spawned":true'* ]]
    [[ "$summary_json" == *'"equipment_change_count":1'* ]]
    [[ "$summary_json" == *'"loot_grant_count":2'* ]]
    [[ "$summary_json" == *'"progression_grant_count":2'* ]]
    [[ "$summary_json" != *'"join_to_start_ms":null'* ]]
    [[ "$summary_json" != *'"activity_duration_ms":null'* ]]

    frozen_output="$(REVENANT_GAME_ADDR="$game_addr" \
      cargo run --quiet -p revenant-frozen-client-v1)"
    echo "$frozen_output"
    [[ "$frozen_output" == *"frozen client V1"* ]]
    [[ "$frozen_output" == *"through compatibility adapter"* ]]

    echo "M14 frozen V1 client compatibility smoke test passed"

    [[ "$(sha256sum archive/clients/v1/src/main.rs | cut -d' ' -f1)" == \
      "4f481e9fc5d22a5ab6d8f2d0a40e2d05dc9aaf92099debdd9dedf59c26f31f72" ]]
    [[ "$(sha256sum archive/clients/v1/Cargo.toml | cut -d' ' -f1)" == \
      "c951c5fe88daa2dd9fb91a4da98ca316fd3923e0bff5332d748db44bce367322" ]]

    kill "$server_pid"
    wait "$server_pid" 2>/dev/null || true
    server_pid=""
    REVENANT_RECONSTRUCTION_ADDR="$game_addr" \
      cargo run --quiet -p revenant-reconstruction-server >"$reconstruction_log" 2>&1 &
    reconstruction_pid=$!
    for _ in {1..100}; do
      if grep -q "reconstructed V1 backend listening" "$reconstruction_log"; then
        break
      fi
      if ! kill -0 "$reconstruction_pid" 2>/dev/null; then
        cat "$reconstruction_log"
        echo "reconstructed V1 backend exited before becoming ready" >&2
        exit 1
      fi
      sleep 0.1
    done
    grep -q "reconstructed V1 backend listening" "$reconstruction_log"

    reconstructed_output="$(REVENANT_GAME_ADDR="$game_addr" \
      cargo run --quiet -p revenant-frozen-client-v1)"
    echo "$reconstructed_output"
    wait "$reconstruction_pid"
    reconstruction_pid=""
    cat "$reconstruction_log"
    [[ "$reconstructed_output" == *"frozen client V1"* ]]
    grep -q "without revenant-gateway" "$reconstruction_log"

    echo "M15 reconstruction experiment smoke test passed"
    echo "M17 playable vertical slice smoke test passed"
    echo "M18 authoritative loot and inventory smoke test passed"
    echo "M19 authoritative progression smoke test passed"
    echo "M20 authoritative equipment and loadout smoke test passed"
    exit 0
fi

cat "$log_file"
echo "gateway healthcheck did not become ready" >&2
exit 1
