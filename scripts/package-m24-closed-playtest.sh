#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
godot_bin="${GODOT_BIN:-$repo_root/.tooling/godot/Godot_v4.7.1-stable_linux.x86_64}"
template_archive="${GODOT_TEMPLATE_ARCHIVE:-$repo_root/.tooling/godot/export-templates/Godot_v4.7.1-stable_export_templates.tpz}"
output_root="${1:-$repo_root/../revenant-local-packages}"
canonical_version="$(tr -d '[:space:]' < "$repo_root/VERSION")"
expected_head="4571892633946a3ef5ef2e1ab1d8bf9fd12f29f6"
template_sha="86409db6200b6f8fd3230989c2d2002851f3dd18acf11d7bdbafddf5a0dd0f72"

if [[ "$canonical_version" != "0.2.0" ]]; then
  echo "M24 package requires VERSION=0.2.0" >&2
  exit 1
fi
if [[ "$(git -C "$repo_root" rev-parse HEAD)" != "$expected_head" ]]; then
  echo "M24 package requires reviewed baseline HEAD $expected_head" >&2
  exit 1
fi
if [[ "$(git -C "$repo_root" rev-parse origin/main)" != "$expected_head" ]]; then
  echo "M24 package requires origin/main at reviewed baseline HEAD" >&2
  exit 1
fi
if [[ ! -x "$godot_bin" ]]; then
  echo "Godot 4.7.1 editor binary is unavailable: $godot_bin" >&2
  exit 1
fi
if [[ ! -f "$template_archive" ]]; then
  echo "Official Godot 4.7.1 export templates are unavailable: $template_archive" >&2
  exit 1
fi
if [[ "$(sha256sum "$template_archive" | cut -d' ' -f1)" != "$template_sha" ]]; then
  echo "Godot export template checksum mismatch" >&2
  exit 1
fi
if ! command -v tar.exe >/dev/null; then
  echo "Windows tar.exe is required to create the closed ZIP package" >&2
  exit 1
fi

mkdir -p "$output_root"
output_root="$(realpath "$output_root")"
case "$output_root/" in
  "$repo_root/"*)
    echo "Closed package output must stay outside the repository" >&2
    exit 1
    ;;
esac

stage_root="$(mktemp -d "$output_root/.m24-package-stage.XXXXXX")"
cleanup() {
  rm -rf "$stage_root"
}
trap cleanup EXIT

source_manifest="$stage_root/SOURCE-SHA256SUMS"
(
  cd "$repo_root"
  git ls-files --cached --others --exclude-standard -- \
    VERSION Cargo.toml Cargo.lock archive client/game infra runtime \
    scripts/activities scripts/package-m24-closed-playtest.sh scripts/playtest \
    tools web/control-panel \
    docs/playtest/m24-participant-instructions.md \
    docs/playtest/m24-operator-runbook.md \
    docs/playtest/m24-consent-form.md \
    docs/playtest/m24-observation-form.md \
    docs/playtest/m24-intervention-log.md \
    docs/playtest/m24-evidence-disposition-form.md \
    docs/playtest/m24-feedback-form.md \
    | LC_ALL=C sort \
    | while IFS= read -r path; do
        sha256sum "$path"
      done
) > "$source_manifest"

if [[ ! -s "$source_manifest" ]]; then
  echo "Source identity manifest is empty" >&2
  exit 1
fi
source_manifest_sha="$(sha256sum "$source_manifest" | cut -d' ' -f1)"
short_head="${expected_head:0:8}"
build_id="m24-b6-${short_head}-${source_manifest_sha:0:12}"
package_name="revenant-m24-closed-${canonical_version}-windows-x86_64-${build_id}"
package_dir="$output_root/$package_name"
archive_path="$output_root/$package_name.zip"

if [[ -e "$package_dir" || -e "$archive_path" || -e "$archive_path.sha256" ]]; then
  echo "Closed package target already exists: $package_name" >&2
  exit 1
fi

template_data="$stage_root/template-data"
template_dir="$template_data/godot/export_templates/4.7.1.stable"
mkdir -p "$template_dir"
tar.exe -xf "$(wslpath -w "$template_archive")" \
  -C "$(wslpath -w "$template_dir")" \
  templates/windows_release_x86_64.exe
mv "$template_dir/templates/windows_release_x86_64.exe" \
  "$template_dir/windows_release_x86_64.exe"
rmdir "$template_dir/templates"
test -s "$template_dir/windows_release_x86_64.exe"

staged_package="$stage_root/$package_name"
export_project="$stage_root/game"
mkdir -p \
  "$staged_package/participant" \
  "$staged_package/operator" \
  "$staged_package/forms" \
  "$staged_package/evidence"
mkdir -p "$export_project"
cp -a "$repo_root/client/game/." "$export_project/"
rm -rf "$export_project/.godot"

env \
  XDG_DATA_HOME="$template_data" \
  XDG_CONFIG_HOME="$stage_root/config" \
  XDG_CACHE_HOME="$stage_root/cache" \
  "$godot_bin" --headless --path "$export_project" \
    --export-release "Windows Desktop" \
    "$staged_package/participant/Revenant.exe"

test -s "$staged_package/participant/Revenant.exe"
test -s "$staged_package/participant/Revenant.pck"
file "$staged_package/participant/Revenant.exe" | grep -q 'PE32+ executable.*x86-64'

cp "$repo_root/docs/playtest/m24-participant-instructions.md" \
  "$staged_package/participant/START-HERE.txt"
cp "$repo_root/docs/playtest/m24-operator-runbook.md" \
  "$staged_package/operator/RUNBOOK.md"
cp "$repo_root/scripts/playtest/Test-RevenantPackage.ps1" \
  "$staged_package/operator/Test-RevenantPackage.ps1"
cp "$repo_root/scripts/playtest/Remove-RevenantReport.ps1" \
  "$staged_package/operator/Remove-RevenantReport.ps1"
sed "s/@BUILD_ID@/$build_id/g" \
  "$repo_root/scripts/playtest/Start-Revenant.ps1.in" \
  > "$staged_package/operator/Start-Revenant.ps1"
cp "$repo_root/docs/playtest/m24-consent-form.md" \
  "$staged_package/forms/CONSENT.md"
cp "$repo_root/docs/playtest/m24-observation-form.md" \
  "$staged_package/forms/OBSERVATION.md"
cp "$repo_root/docs/playtest/m24-intervention-log.md" \
  "$staged_package/forms/INTERVENTION.md"
cp "$repo_root/docs/playtest/m24-evidence-disposition-form.md" \
  "$staged_package/forms/EVIDENCE-DISPOSITION.md"
cp "$repo_root/docs/playtest/m24-feedback-form.md" \
  "$staged_package/forms/FEEDBACK.md"
cp "$source_manifest" "$staged_package/evidence/SOURCE-SHA256SUMS"

cat > "$staged_package/evidence/BUILD-IDENTITY.txt" <<EOF
package_id=$build_id
package_kind=m24-closed-playtest-unpublished
product_version=$canonical_version
baseline_head=$expected_head
baseline_branch=main
source_tree_state=reviewed-uncommitted
source_manifest_sha256=$source_manifest_sha
godot_version=4.7.1.stable.official.a13da4feb
godot_template_sha256=$template_sha
platform=windows-x86_64
protocol=v2
reconnect_or_resume=false
public_distribution=false
code_signed=false
EOF

find "$staged_package" -type d -exec chmod 0755 {} +
find "$staged_package" -type f -exec chmod 0644 {} +
(
  cd "$staged_package"
  find . -type f ! -name SHA256SUMS -print0 \
    | LC_ALL=C sort -z \
    | xargs -0 sha256sum > SHA256SUMS
  sha256sum --check --quiet SHA256SUMS
)

if ! cp -a "$staged_package" "$package_dir"; then
  rm -rf "$package_dir"
  echo "Could not promote the closed package into its external output directory" >&2
  exit 1
fi
(
  cd "$package_dir"
  sha256sum --check --quiet SHA256SUMS
)
(
  cd "$output_root"
  tar.exe -a -cf "$package_name.zip" "$package_name"
  sha256sum "$package_name.zip" > "$package_name.zip.sha256"
)

echo "M24_PACKAGE_ID=$build_id"
echo "M24_PACKAGE_DIR=$package_dir"
echo "M24_PACKAGE_ARCHIVE=$archive_path"
echo "M24_PACKAGE_SHA256=$(sha256sum "$archive_path" | cut -d' ' -f1)"
