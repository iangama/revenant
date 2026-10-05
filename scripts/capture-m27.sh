#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
output_dir="${1:-$repo_root/docs/art/m27/captures}"
godot_bin="${GODOT_BIN:-$repo_root/.tooling/godot/Godot_v4.7.1-stable_linux.x86_64}"
runtime_dir="$(mktemp -d)"
project_dir="$runtime_dir/project"
gateway_log="$runtime_dir/gateway.log"
health_addr="127.0.0.1:18082"
game_addr="127.0.0.1:17002"

cleanup() {
  if [[ -n "${gateway_pid:-}" ]]; then
    kill "$gateway_pid" 2>/dev/null || true
    wait "$gateway_pid" 2>/dev/null || true
  fi
  rm -rf "$runtime_dir"
}
trap cleanup EXIT

if [[ ! -x "$godot_bin" ]]; then
  echo "Godot executable is unavailable: $godot_bin" >&2
  exit 1
fi

mkdir -p "$output_dir" "$project_dir"
(
  cd "$repo_root/client/game"
  tar --exclude='./.godot' -cf - .
) | (
  cd "$project_dir"
  tar -xf -
)

XDG_DATA_HOME="$runtime_dir/data" \
XDG_CONFIG_HOME="$runtime_dir/config" \
XDG_CACHE_HOME="$runtime_dir/cache" \
  "$godot_bin" --headless --editor --path "$project_dir" --quit

cd "$repo_root"
REVENANT_BIND_ADDR="$health_addr" \
REVENANT_GAME_ADDR="$game_addr" \
REVENANT_EXPECTED_PLAYERS=1 \
  cargo run --quiet -p revenant-gateway >"$gateway_log" 2>&1 &
gateway_pid=$!
for _attempt in {1..180}; do
  if curl --fail --silent "http://$health_addr/health" >/dev/null; then
    break
  fi
  if ! kill -0 "$gateway_pid" 2>/dev/null; then
    cat "$gateway_log"
    echo "M27 capture gateway exited before readiness" >&2
    exit 1
  fi
  sleep 0.2
done
curl --fail --silent "http://$health_addr/health" >/dev/null

XDG_DATA_HOME="$runtime_dir/data" \
XDG_CONFIG_HOME="$runtime_dir/config" \
XDG_CACHE_HOME="$runtime_dir/cache" \
REVENANT_GAME_HOST="127.0.0.1" \
REVENANT_GAME_PORT="17002" \
REVENANT_GAME_USERNAME="m27-route-capture" \
REVENANT_VALIDATE_ROUTE_FLOW="stabilize" \
REVENANT_CAPTURE_M27_DIR="$output_dir" \
  timeout 45s "$godot_bin" --path "$project_dir"

for filename in \
  01-route-choice.png \
  02-route-accepted.png \
  03-route-summary.png; do
  test -s "$output_dir/$filename"
done

(
  cd "$output_dir"
  sha256sum \
    01-route-choice.png \
    02-route-accepted.png \
    03-route-summary.png > SHA256SUMS
  sha256sum --check --quiet SHA256SUMS
)

echo "M27 authoritative choice/acceptance/summary captures created in $output_dir"
