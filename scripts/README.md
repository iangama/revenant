# Scripts

Restricted Lua activity definitions live under this boundary. They define objectives, triggers, completion loot, and experience rewards. Item identifiers and positive quantities are validated by the inventory domain; experience is bounded by the progression domain. Scripts cannot create arbitrary items or author resulting levels.

`check-version.sh` enforces `VERSION` as the canonical product version across Cargo, the Inspector package and lockfile, and the current Godot client build. `release.sh` refuses to package a mismatched tree.

`capture-m21.sh` runs the deterministic Godot visual validation with a graphical renderer and writes the three M21 review shots plus `SHA256SUMS`. Set `GODOT_BIN` when the repository-local Godot executable is unavailable. The default output is `docs/art/m21/captures`; pass another directory as the first argument for a disposable review run.

`measure-m22.sh` performs the opt-in graphical Master-bus and frame-time measurement for the M22 presentation. `capture-m22.sh` produces Entry, Settings, onboarding, and runtime PNGs plus a short MJPEG/PCM AVI and verified `SHA256SUMS`. Its default output is `docs/art/m22/captures`; pass another directory for a disposable run.
