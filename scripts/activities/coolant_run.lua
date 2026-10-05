return {
    id = "coolant_run",
    kind = "field_excursion",
    reward = { item_id = "relay_core_fragment", quantity = 1 },
    progression = { experience = 100 },
    objectives = {
        { id = "coolant_intake", kind = "ReachArea", state = "Active", target = 1 },
        { id = "coolant_transfer", kind = "ReachArea", state = "Pending", target = 1 },
        { id = "coolant_delivery", kind = "ReachArea", state = "Pending", target = 1 },
    },
    triggers = {
        { event = "AreaReached", subject = "coolant_intake", complete = { "coolant_intake" }, activate = { "coolant_transfer" } },
        { event = "AreaReached", subject = "coolant_transfer", complete = { "coolant_transfer" }, activate = { "coolant_delivery" } },
        { event = "AreaReached", subject = "coolant_delivery", complete = { "coolant_delivery" }, complete_activity = true },
    },
}
