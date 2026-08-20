# Lua activity schema

An activity script returns a table with `id`, ordered `objectives`, and ordered `triggers`. Objectives define `id`, `kind`, initial `state`, and `target`. Triggers match an `event` and `subject`, then may complete or activate objectives, open a door, request a boss spawn, or complete the activity.

Supported objective kinds are `KillActors`, `ReachArea`, and `Boss`. Supported trigger events are `ActorGroupDead` and `AreaReached`.

See `scripts/activities/relay_awakening.lua` for the canonical M9 definition.
