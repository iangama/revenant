#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
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

XDG_DATA_HOME="$runtime_dir/data" \
XDG_CONFIG_HOME="$runtime_dir/config" \
XDG_CACHE_HOME="$runtime_dir/cache" \
REVENANT_VALIDATE_SLICE=1 \
REVENANT_MEASURE_M22=1 \
  "$godot_bin" --path "$repo_root/client/game"

