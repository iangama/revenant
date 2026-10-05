#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
output_dir="${1:-$repo_root/docs/art/m25/captures}"
godot_bin="${GODOT_BIN:-$repo_root/.tooling/godot/Godot_v4.7.1-stable_linux.x86_64}"
runtime_dir="$(mktemp -d)"
project_dir="$runtime_dir/project"

cleanup() {
  rm -rf "$runtime_dir"
}
trap cleanup EXIT

if [[ ! -x "$godot_bin" ]]; then
  echo "Godot executable is unavailable: $godot_bin" >&2
  exit 1
fi

mkdir -p "$output_dir"
mkdir -p "$project_dir"
(
  cd "$repo_root/client/game"
  tar --exclude='./.godot' -cf - .
) | (
  cd "$project_dir"
  tar -xf -
)

# Rebuild the temporary import cache before the graphical run. The checked-in
# .import sidecars intentionally point at project-local generated artifacts,
# which do not exist in a fresh isolated copy yet.
XDG_DATA_HOME="$runtime_dir/data" \
XDG_CONFIG_HOME="$runtime_dir/config" \
XDG_CACHE_HOME="$runtime_dir/cache" \
  "$godot_bin" --headless --editor --path "$project_dir" --quit

XDG_DATA_HOME="$runtime_dir/data" \
XDG_CONFIG_HOME="$runtime_dir/config" \
XDG_CACHE_HOME="$runtime_dir/cache" \
REVENANT_VALIDATE_SLICE=1 \
REVENANT_CAPTURE_M25_DIR="$output_dir" \
  "$godot_bin" --path "$project_dir"

for filename in \
  01-local-attempt.png \
  02-server-confirmed-hit.png; do
  test -s "$output_dir/$filename"
done

(
  cd "$output_dir"
  sha256sum \
    01-local-attempt.png \
    02-server-confirmed-hit.png > SHA256SUMS
  sha256sum --check --quiet SHA256SUMS
)

echo "M25 local-attempt/authoritative-confirmation A/B captures created in $output_dir"
