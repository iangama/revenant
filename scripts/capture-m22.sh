#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
output_dir="${1:-$repo_root/docs/art/m22/captures}"
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
REVENANT_MEASURE_M22=1 \
REVENANT_CAPTURE_M22_ENTRY="$output_dir/01-entry.png" \
REVENANT_CAPTURE_M22_SETTINGS="$output_dir/02-settings.png" \
REVENANT_CAPTURE_M22_ONBOARDING="$output_dir/03-onboarding.png" \
REVENANT_CAPTURE_M22_RUNTIME="$output_dir/04-runtime.png" \
  "$godot_bin" \
    --path "$repo_root/client/game" \
    --fixed-fps 30 \
    --write-movie "$output_dir/05-m22-review.avi"

for filename in \
  01-entry.png \
  02-settings.png \
  03-onboarding.png \
  04-runtime.png \
  05-m22-review.avi; do
  test -s "$output_dir/$filename"
done

(
  cd "$output_dir"
  sha256sum \
    01-entry.png \
    02-settings.png \
    03-onboarding.png \
    04-runtime.png \
    05-m22-review.avi > SHA256SUMS
  sha256sum --check --quiet SHA256SUMS
)

echo "M22 review captures created in $output_dir"
