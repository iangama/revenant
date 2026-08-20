# M6 — AI

Status: complete and validated locally.

The server runs a finite-state controller through `Idle`, `Detect`, `Chase`, `Attack`, and `Dead`. The `relay-drone` detects within 12 units, chases one grid unit per tick until two-unit attack range, then deals 10 server-authoritative damage. Movement replicates through `ActorUpdate` and damage through `DamageApplied`.
