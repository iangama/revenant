return {
    id = "meridian_survey",
    kind = "field_excursion",
    reward = { item_id = "relay_core_fragment", quantity = 1 },
    progression = { experience = 100 },
    objectives = {
        { id = "meridian_arrival", kind = "ReachArea", state = "Active", target = 1 },
        { id = "meridian_lens", kind = "ReachArea", state = "Pending", target = 1 },
        { id = "meridian_gallery", kind = "ReachArea", state = "Pending", target = 1 },
    },
    triggers = {
        { event = "AreaReached", subject = "meridian_arrival", complete = { "meridian_arrival" }, activate = { "meridian_lens" } },
        { event = "AreaReached", subject = "meridian_lens", complete = { "meridian_lens" }, activate = { "meridian_gallery" } },
        { event = "AreaReached", subject = "meridian_gallery", complete = { "meridian_gallery" }, complete_activity = true },
    },
}
