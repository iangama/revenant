return {
    id = "prism_core",
    kind = "campaign_chapter",
    reward = { item_id = "relay_core_fragment", quantity = 1 },
    progression = { experience = 100 },
    objectives = {
        { id = "prism_arrival", kind = "ReachArea", state = "Active", target = 1 },
        { id = "prism_guard", kind = "KillActors", state = "Pending", target = 1 },
        { id = "prism_shutdown", kind = "ReachArea", state = "Pending", target = 1 },
        { id = "prism_return", kind = "ReachArea", state = "Pending", target = 1 },
    },
    triggers = {
        { event = "AreaReached", subject = "prism_arrival", complete = { "prism_arrival" }, activate = { "prism_guard" } },
        { event = "ActorGroupDead", subject = "prism_guard", complete = { "prism_guard" }, activate = { "prism_shutdown" } },
        { event = "AreaReached", subject = "prism_shutdown", complete = { "prism_shutdown" }, activate = { "prism_return" } },
        { event = "AreaReached", subject = "prism_return", complete = { "prism_return" }, complete_activity = true },
    },
}
