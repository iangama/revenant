# M4 — Actor Runtime

Status: complete and validated locally.

`ActorRegistry` owns explicit spawn, position update, lookup, and destroy operations. After world join, the server creates the player actor and one `relay-drone` enemy, then replicates both through `ActorSpawn`. The Godot client materializes server-provided actors as simple 3D meshes. Combat and AI remain outside M4.
