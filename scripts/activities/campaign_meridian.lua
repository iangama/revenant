return {
    id = "meridian_readings",
    kind = "field_excursion",
    reward = { item_id = "relay_core_fragment", quantity = 1 },
    progression = { experience = 100 },
    objectives = {
        { id = "campaign_meridian", kind = "ReachArea", state = "Active", target = 1 },
    },
    triggers = {
        { event = "AreaReached", subject = "campaign_meridian", complete = { "campaign_meridian" }, complete_activity = true },
    },
}
