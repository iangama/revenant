return {
    id = "relay_awakening",
    authoring_revision = "m27-v1",
    reward = { item_id = "relay_core_fragment", quantity = 1 },
    progression = { experience = 100 },
    objectives = {
        { id = "clear_drone_group", kind = "KillActors", state = "Active", target = 1 },
        { id = "reach_relay_door", kind = "ReachArea", state = "Pending", target = 1 },
        { id = "reach_relay_stabilizer", kind = "ReachArea", state = "Pending", target = 1 },
        { id = "defeat_warden", kind = "Boss", state = "Pending", target = 1 },
    },
    triggers = {
        {
            event = "AreaReached",
            subject = "relay_stabilizer",
            complete = { "reach_relay_stabilizer" },
            activate = { "reach_relay_door" },
        },
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
    routes = {
        {
            id = "breach",
            duration_ms = 90000,
            reward = { item_id = "relay_core_fragment", quantity = 2 },
            progression = { experience = 100 },
            objective_path = {
                "clear_drone_group",
                "reach_relay_door",
                "defeat_warden",
            },
            events = {
                {
                    id = "overcharged_armor",
                    warden_health_basis_points = 12000,
                    warden_counter_damage = 15,
                },
                {
                    id = "arc_surge",
                    warden_health_basis_points = 10000,
                    warden_counter_damage = 20,
                },
            },
        },
        {
            id = "stabilize",
            duration_ms = 90000,
            reward = { item_id = "relay_core_fragment", quantity = 1 },
            progression = { experience = 150 },
            objective_path = {
                "clear_drone_group",
                "reach_relay_stabilizer",
                "reach_relay_door",
                "defeat_warden",
            },
            events = {
                {
                    id = "shielded_channel",
                    warden_health_basis_points = 10000,
                    warden_counter_damage = 10,
                },
                {
                    id = "residual_feedback",
                    warden_health_basis_points = 11000,
                    warden_counter_damage = 15,
                },
            },
        },
    },
}
