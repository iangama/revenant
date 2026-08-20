# M5 — Combat

Status: complete and validated locally.

Clients send only `AttackIntent(target_actor_id)`. The server validates actor roles, target existence, six-unit range, and a 250 ms cooldown. A valid basic attack deals 40 damage. The initial 100 HP `relay-drone` dies after three valid hits; the server emits `DamageApplied` and then `ActorDestroy`.
