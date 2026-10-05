# M24 Block 5 review captures

These captures are representative visual evidence for the bounded display and
first-contact matrix in `docs/playtest/m24-display-first-contact-matrix.md`.
They are generated from the local development build, contain no participant
identity or external telemetry, and are not authoritative gameplay records.

- `01-entry-clean-1280x720.png`: clean-data entry shell at the authored size.
- `02-settings-saved-1366x768.png`: saved Compact/muted settings and keyboard
  focus at 1366×768; the rendered 16:9 surface is 1365×768 by aspect rounding.
- `03-runtime-combined-2560x1440.png`: scaled HUD with mute and Reduced Flash.
- `04-runtime-fullscreen-1920x1080.png`: native fullscreen with Guidance Off.
- `05-failure-retry-1280x720.png`: immediate-refusal Retry state.
- `06-keyboard-completion-1920x1080.png`: real keyboard-only completion and
  authoritative reward projection.

`captures/SHA256SUMS` is the review manifest. Regenerate the matrix with:

```bash
GODOT_BIN=.tooling/godot/Godot_v4.7.1-stable_linux.x86_64 \
  bash scripts/validate-m24-display.sh /tmp/revenant-m24-display-review
```

The six retained files are selected review artifacts; the command produces and
hashes all 24 entry, settings, onboarding, and runtime captures.
