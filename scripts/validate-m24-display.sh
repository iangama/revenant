#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
godot_bin="${GODOT_BIN:-$repo_root/.tooling/godot/Godot_v4.7.1-stable_linux.x86_64}"
output_dir="${1:-$(mktemp -d /tmp/revenant-m24-display.XXXXXX)}"
display_driver="${M24_DISPLAY_DRIVER:-wayland}"
rendering_driver="${M24_RENDERING_DRIVER:-opengl3}"

if [[ ! -x "$godot_bin" ]]; then
  echo "Godot executable is unavailable: $godot_bin" >&2
  exit 1
fi

mkdir -p "$output_dir"

png_dimensions() {
  file "$1" | sed -n 's/.*PNG image data, \([0-9][0-9]*\) x \([0-9][0-9]*\).*/\1x\2/p'
}

assert_consistent_capture_dimensions() {
  local case_dir="$1"
  local baseline
  baseline="$(png_dimensions "$case_dir/entry.png")"
  if [[ -z "$baseline" ]]; then
    echo "Could not read entry capture dimensions for $case_dir" >&2
    return 1
  fi
  for capture in settings onboarding runtime; do
    local actual
    actual="$(png_dimensions "$case_dir/$capture.png")"
    if [[ "$actual" != "$baseline" ]]; then
      echo "$capture capture changed from $baseline to $actual in $case_dir" >&2
      return 1
    fi
  done
}

run_case() {
  local case_id="$1"
  local resolution="$2"
  local fixture="$3"
  local guidance="$4"
  local muted="$5"
  local reduced_flash="$6"
  local display_mode="$7"
  local test_switch="$8"
  local case_dir="$output_dir/$case_id"
  local user_dir="$case_dir/xdg/data/godot/app_userdata/Revenant"

  mkdir -p "$user_dir" "$case_dir/xdg/config" "$case_dir/xdg/cache"
  if [[ -n "$fixture" ]]; then
    cp "$repo_root/tests/fixtures/$fixture" "$user_dir/revenant-settings.cfg"
  fi

  local -a expected_size=()
  if [[ "$display_mode" == "Windowed" ]]; then
    expected_size=("REVENANT_EXPECT_WINDOW_SIZE=$resolution")
  fi

  env \
    XDG_DATA_HOME="$case_dir/xdg/data" \
    XDG_CONFIG_HOME="$case_dir/xdg/config" \
    XDG_CACHE_HOME="$case_dir/xdg/cache" \
    REVENANT_VALIDATE_SLICE=1 \
    REVENANT_EXPECT_DISPLAY_MODE="$display_mode" \
    REVENANT_EXPECT_GUIDANCE_MODE="$guidance" \
    REVENANT_EXPECT_MUTED="$muted" \
    REVENANT_EXPECT_REDUCED_FLASH="$reduced_flash" \
    REVENANT_TEST_DISPLAY_SWITCH="$test_switch" \
    REVENANT_CAPTURE_M22_ENTRY="$case_dir/entry.png" \
    REVENANT_CAPTURE_M22_SETTINGS="$case_dir/settings.png" \
    REVENANT_CAPTURE_M22_ONBOARDING="$case_dir/onboarding.png" \
    REVENANT_CAPTURE_M22_RUNTIME="$case_dir/runtime.png" \
    "${expected_size[@]}" \
    timeout 45s "$godot_bin" \
      --display-driver "$display_driver" \
      --rendering-driver "$rendering_driver" \
      --audio-driver Dummy \
      --path "$repo_root/client/game" \
      --resolution "$resolution" \
      --position 20,20 >"$case_dir/run.log" 2>&1

  grep -q "M24 display and first-contact validated" "$case_dir/run.log"
  for capture in entry settings onboarding runtime; do
    test -s "$case_dir/$capture.png"
  done
  assert_consistent_capture_dimensions "$case_dir"
  echo "$case_id passed"
  grep "M24 display case validated" "$case_dir/run.log"
}

run_case "01-windowed-clean-1280x720" "1280x720" "" "Full" 0 0 "Windowed" 0
run_case "02-windowed-saved-1366x768" "1366x768" "m24-settings-compact-muted.cfg" "Compact" 1 0 "Windowed" 1
run_case "03-windowed-malformed-1920x1080" "1920x1080" "m24-settings-malformed.cfg" "Full" 0 0 "Windowed" 0
run_case "04-windowed-combined-2560x1440" "2560x1440" "m24-settings-full-combined.cfg" "Full" 1 1 "Windowed" 0
run_case "05-fullscreen-native" "1280x720" "m24-settings-fullscreen-off.cfg" "Off" 0 1 "Fullscreen" 1

second_case="$output_dir/06-second-launch-1366x768"
mkdir -p "$second_case"
cp -a "$output_dir/02-windowed-saved-1366x768/xdg" "$second_case/xdg"
env \
  XDG_DATA_HOME="$second_case/xdg/data" \
  XDG_CONFIG_HOME="$second_case/xdg/config" \
  XDG_CACHE_HOME="$second_case/xdg/cache" \
  REVENANT_VALIDATE_SLICE=1 \
  REVENANT_EXPECT_WINDOW_SIZE=1366x768 \
  REVENANT_EXPECT_DISPLAY_MODE=Windowed \
  REVENANT_EXPECT_GUIDANCE_MODE=Compact \
  REVENANT_EXPECT_MUTED=1 \
  REVENANT_EXPECT_REDUCED_FLASH=0 \
  REVENANT_CAPTURE_M22_ENTRY="$second_case/entry.png" \
  REVENANT_CAPTURE_M22_SETTINGS="$second_case/settings.png" \
  REVENANT_CAPTURE_M22_ONBOARDING="$second_case/onboarding.png" \
  REVENANT_CAPTURE_M22_RUNTIME="$second_case/runtime.png" \
  timeout 45s "$godot_bin" \
    --display-driver "$display_driver" \
    --rendering-driver "$rendering_driver" \
    --audio-driver Dummy \
    --path "$repo_root/client/game" \
    --resolution 1366x768 \
    --position 20,20 >"$second_case/run.log" 2>&1
grep -q "M24 display and first-contact validated" "$second_case/run.log"
for capture in entry settings onboarding runtime; do
  test -s "$second_case/$capture.png"
done
assert_consistent_capture_dimensions "$second_case"
echo "06-second-launch-1366x768 passed"
grep "M24 display case validated" "$second_case/run.log"

(
  cd "$output_dir"
  find . -type f -name '*.png' -print0 | sort -z | xargs -0 sha256sum > SHA256SUMS
  sha256sum --check --quiet SHA256SUMS
)

echo "M24 display matrix created in $output_dir"
