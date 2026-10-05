return {
    id = "the_breach",
    kind = "campaign_chapter",
    reward = { item_id = "relay_core_fragment", quantity = 1 },
    progression = { experience = 100 },
    objectives = {
        { id = "breach_door", kind = "ReachArea", state = "Active", target = 1 },
        { id = "breach_guard", kind = "KillActors", state = "Pending", target = 1 },
        { id = "breach_stabilizer", kind = "ReachArea", state = "Pending", target = 1 },
        { id = "breach_core", kind = "ReachArea", state = "Pending", target = 1 },
    },
    triggers = {
        { event = "AreaReached", subject = "breach_door", complete = { "breach_door" }, activate = { "breach_guard" } },
        { event = "ActorGroupDead", subject = "breach_guard", complete = { "breach_guard" }, activate = { "breach_stabilizer" } },
        { event = "AreaReached", subject = "breach_stabilizer", complete = { "breach_stabilizer" }, activate = { "breach_core" } },
        { event = "AreaReached", subject = "breach_core", complete = { "breach_core" }, complete_activity = true },
    },
}
