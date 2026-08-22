return {
    id = "relay_awakening",
    reward = { item_id = "relay_core_fragment", quantity = 1 },
    objectives = {
        { id = "clear_drone_group", kind = "KillActors", state = "Active", target = 1 },
        { id = "reach_relay_door", kind = "ReachArea", state = "Pending", target = 1 },
        { id = "defeat_warden", kind = "Boss", state = "Pending", target = 1 },
    },
    triggers = {
        {
            event = "ActorGroupDead",
            subject = "relay_drones",
            complete = { "clear_drone_group" },
            activate = { "reach_relay_door" },
        },
        {
            event = "AreaReached",
            subject = "relay_door",
            complete = { "reach_relay_door" },
            activate = { "defeat_warden" },
            open_door = "relay_core",
            spawn_boss = "warden",
        },
        {
            event = "ActorGroupDead",
            subject = "warden",
            complete = { "defeat_warden" },
            complete_activity = true,
        },
    },
}
