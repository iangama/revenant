return {
    id = "meridian_return",
    kind = "field_excursion",
    reward = { item_id = "relay_core_fragment", quantity = 1 },
    progression = { experience = 100 },
    objectives = {
        { id = "meridian_log", kind = "ReachArea", state = "Active", target = 1 },
        { id = "meridian_return", kind = "ReachArea", state = "Pending", target = 1 },
    },
    triggers = {
        { event = "AreaReached", subject = "meridian_log", complete = { "meridian_log" }, activate = { "meridian_return" } },
        { event = "AreaReached", subject = "meridian_return", complete = { "meridian_return" }, complete_activity = true },
    },
}
