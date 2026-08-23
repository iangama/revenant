#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

canonical_version="$(tr -d '[:space:]' < VERSION)"
if [[ ! "$canonical_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "VERSION must use MAJOR.MINOR.PATCH" >&2
  exit 1
fi

require_version() {
  local label="$1"
  local actual="$2"
  if [[ "$actual" != "$canonical_version" ]]; then
    echo "$label version $actual does not match VERSION ($canonical_version)" >&2
    exit 1
  fi
}

workspace_version="$(sed -n 's/^version = "\([0-9][0-9.]*\)"$/\1/p' Cargo.toml | head -1)"
inspector_version="$(sed -n 's/^[[:space:]]*"version": "\([0-9][0-9.]*\)",$/\1/p' web/control-panel/package.json | head -1)"
mapfile -t inspector_lock_versions < <(
  sed -n 's/^[[:space:]]*"version": "\([0-9][0-9.]*\)",$/\1/p' \
    web/control-panel/package-lock.json | head -2
)
godot_version="$(sed -n 's/^[[:space:]]*"client_build": "\([0-9][0-9.]*\)",$/\1/p' client/game/main.gd | head -1)"

require_version "Rust workspace" "$workspace_version"
require_version "Inspector package" "$inspector_version"
if [[ "${#inspector_lock_versions[@]}" -ne 2 ]]; then
  echo "Inspector lockfile must expose root and package versions" >&2
  exit 1
fi
require_version "Inspector lockfile root" "${inspector_lock_versions[0]}"
require_version "Inspector lockfile package" "${inspector_lock_versions[1]}"
require_version "Godot client build" "$godot_version"

echo "version consistency verified: $canonical_version"
