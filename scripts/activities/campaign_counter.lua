return {
    id = "counter_signal",
    kind = "campaign_chapter",
    reward = { item_id = "relay_core_fragment", quantity = 1 },
    progression = { experience = 100 },
    objectives = {
        { id = "counter_approach", kind = "ReachArea", state = "Active", target = 1 },
        { id = "counter_support", kind = "KillActors", state = "Pending", target = 1 },
        { id = "counter_transmission", kind = "ReachArea", state = "Pending", target = 1 },
        { id = "counter_bastion", kind = "ReachArea", state = "Pending", target = 1 },
        { id = "counter_guard", kind = "KillActors", state = "Pending", target = 1 },
        { id = "counter_emitter", kind = "ReachArea", state = "Pending", target = 1 },
    },
    triggers = {
        { event = "AreaReached", subject = "counter_approach", complete = { "counter_approach" }, activate = { "counter_support" } },
        { event = "ActorGroupDead", subject = "counter_support", complete = { "counter_support" }, activate = { "counter_transmission" } },
        { event = "AreaReached", subject = "counter_transmission", complete = { "counter_transmission" }, activate = { "counter_bastion" } },
        { event = "AreaReached", subject = "counter_bastion", complete = { "counter_bastion" }, activate = { "counter_guard" } },
        { event = "ActorGroupDead", subject = "counter_guard", complete = { "counter_guard" }, activate = { "counter_emitter" } },
        { event = "AreaReached", subject = "counter_emitter", complete = { "counter_emitter" }, complete_activity = true },
    },
}
