#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
version="${1:-0.2.0}"
output_root="${2:-$repo_root/release}"
release_dir="$output_root/revenant-$version-linux-x86_64"
release_notes="$repo_root/docs/release-$version.md"

if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "release version must use MAJOR.MINOR.PATCH" >&2
  exit 1
fi
workspace_version="$(sed -n 's/^version = "\([0-9][0-9.]*\)"$/\1/p' Cargo.toml | head -1)"
inspector_version="$(sed -n 's/^[[:space:]]*"version": "\([0-9][0-9.]*\)",$/\1/p' web/control-panel/package.json | head -1)"
if [[ "$version" != "$workspace_version" || "$version" != "$inspector_version" ]]; then
  echo "release version $version does not match Rust ($workspace_version) and Inspector ($inspector_version)" >&2
  exit 1
fi
if [[ ! -f "$release_notes" ]]; then
  echo "release notes do not exist: $release_notes" >&2
  exit 1
fi
if [[ -e "$release_dir" || -e "$release_dir.tar.gz" ]]; then
  echo "release target already exists: $release_dir" >&2
  exit 1
fi

cd "$repo_root"
cargo build --locked --release \
  -p revenant-gateway \
  -p revenant-cli \
  -p revenant-frozen-client-v1 \
  -p revenant-reconstruction-server
npm --prefix web/control-panel ci
npm --prefix web/control-panel run build

mkdir -p \
  "$release_dir/bin" \
  "$release_dir/inspector" \
  "$release_dir/game" \
  "$release_dir/scripts/activities" \
  "$release_dir/docs"
cp target/release/revenant-gateway "$release_dir/bin/"
cp target/release/revenant "$release_dir/bin/"
cp target/release/revenant-client-v1 "$release_dir/bin/"
cp target/release/revenant-reconstructed-v1 "$release_dir/bin/"
cp -R web/control-panel/dist/. "$release_dir/inspector/"
cp client/game/project.godot client/game/main.gd client/game/main.tscn "$release_dir/game/"
cp scripts/activities/relay_awakening.lua "$release_dir/scripts/activities/"
cp README.md "$release_notes" docs/operations/postgresql-backup.md "$release_dir/docs/"

# Normalize modes so packaging on NTFS, ext4, or a CI filesystem produces the same archive.
find "$release_dir" -type d -exec chmod 0755 {} +
find "$release_dir" -type f -exec chmod 0644 {} +
chmod 0755 "$release_dir"/bin/*

(
  cd "$release_dir"
  find . -type f ! -name SHA256SUMS -print0 \
    | sort -z \
    | xargs -0 sha256sum > SHA256SUMS
  sha256sum --check --quiet SHA256SUMS
)

archive_stage="$(mktemp -d)"
cleanup_archive_stage() {
  rm -rf "$archive_stage"
}
trap cleanup_archive_stage EXIT
cp -R "$release_dir" "$archive_stage/"
staged_release="$archive_stage/$(basename "$release_dir")"
find "$staged_release" -type d -exec chmod 0755 {} +
find "$staged_release" -type f -exec chmod 0644 {} +
chmod 0755 "$staged_release"/bin/*

tar --sort=name \
  --mtime='UTC 1970-01-01' \
  --owner=0 --group=0 --numeric-owner \
  -C "$archive_stage" \
  -cf - "$(basename "$release_dir")" \
  | gzip -n > "$release_dir.tar.gz"
(
  cd "$output_root"
  sha256sum "$(basename "$release_dir").tar.gz" \
    > "$(basename "$release_dir").tar.gz.sha256"
)

echo "release package created: $release_dir.tar.gz"
