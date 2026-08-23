# Scripts

Restricted Lua activity definitions live under this boundary. They define objectives, triggers, completion loot, and experience rewards. Item identifiers and positive quantities are validated by the inventory domain; experience is bounded by the progression domain. Scripts cannot create arbitrary items or author resulting levels.

`check-version.sh` enforces `VERSION` as the canonical product version across Cargo, the Inspector package and lockfile, and the current Godot client build. `release.sh` refuses to package a mismatched tree.
