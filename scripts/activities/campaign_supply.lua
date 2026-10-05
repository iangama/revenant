return {
    id = "broken_supply_line",
    kind = "campaign_chapter",
    reward = { item_id = "relay_core_fragment", quantity = 1 },
    progression = { experience = 100 },
    objectives = {
        { id = "supply_route", kind = "ReachArea", state = "Active", target = 1 },
        { id = "supply_guard", kind = "KillActors", state = "Pending", target = 1 },
        { id = "supply_cell", kind = "ReachArea", state = "Pending", target = 1 },
        { id = "supply_delivery", kind = "ReachArea", state = "Pending", target = 1 },
    },
    triggers = {
        { event = "AreaReached", subject = "supply_route", complete = { "supply_route" }, activate = { "supply_guard" } },
        { event = "ActorGroupDead", subject = "supply_guard", complete = { "supply_guard" }, activate = { "supply_cell" } },
        { event = "AreaReached", subject = "supply_cell", complete = { "supply_cell" }, activate = { "supply_delivery" } },
        { event = "AreaReached", subject = "supply_delivery", complete = { "supply_delivery" }, complete_activity = true },
    },
}
