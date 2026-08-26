#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
output_dir="${1:-$repo_root/docs/art/m21/captures}"
godot_bin="${GODOT_BIN:-$repo_root/.tooling/godot/Godot_v4.7.1-stable_linux.x86_64}"
runtime_dir="$(mktemp -d)"

cleanup() {
  rm -rf "$runtime_dir"
}
trap cleanup EXIT

if [[ ! -x "$godot_bin" ]]; then
  echo "Godot executable is unavailable: $godot_bin" >&2
  exit 1
fi

mkdir -p "$output_dir"

XDG_DATA_HOME="$runtime_dir/data" \
XDG_CONFIG_HOME="$runtime_dir/config" \
XDG_CACHE_HOME="$runtime_dir/cache" \
REVENANT_VALIDATE_SLICE=1 \
REVENANT_CAPTURE_M21_DIR="$output_dir" \
  "$godot_bin" --path "$repo_root/client/game"

for filename in \
  01-relay-hub-overview.png \
  02-enemy-telegraphs.png \
  03-combat-feedback.png; do
  test -s "$output_dir/$filename"
done

(
  cd "$output_dir"
  sha256sum \
    01-relay-hub-overview.png \
    02-enemy-telegraphs.png \
    03-combat-feedback.png > SHA256SUMS
  sha256sum --check --quiet SHA256SUMS
)

echo "M21 review captures created in $output_dir"
