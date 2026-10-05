use super::{
    aggregate_modifiers, canonicalize_loadout, scale_basis_points, validate_catalog,
    BaseCombatProfile, EffectiveCombatProfile, ModuleDefinition, ModuleError, ModuleFamily,
    ModuleId, ResolvedBuild, BUILD_CATALOG_REVISION, CONTENT_MODULE_CATALOG,
};

pub const BUILD_MODULE_CATALOG: [ModuleDefinition; 10] = [
    CONTENT_MODULE_CATALOG[0],
    CONTENT_MODULE_CATALOG[1],
    CONTENT_MODULE_CATALOG[2],
    CONTENT_MODULE_CATALOG[3],
    CONTENT_MODULE_CATALOG[4],
    CONTENT_MODULE_CATALOG[5],
    ModuleDefinition {
        module_id: ModuleId::BreachShunt,
        family: ModuleFamily::Breach,
        recipe_fragments: 1,
        damage_basis_points: 3_000,
        cooldown_basis_points: 3_500,
        range_delta: -1,
        max_health_delta: 0,
    },
    ModuleDefinition {
        module_id: ModuleId::StandoffOptic,
        family: ModuleFamily::Optic,
        recipe_fragments: 1,
        damage_basis_points: -1_000,
        cooldown_basis_points: 2_000,
        range_delta: 3,
        max_health_delta: 0,
    },
    ModuleDefinition {
        module_id: ModuleId::SkirmishDrive,
        family: ModuleFamily::Skirmish,
        recipe_fragments: 1,
        damage_basis_points: 0,
        cooldown_basis_points: -2_500,
        range_delta: 0,
        max_health_delta: -20,
    },
    ModuleDefinition {
        module_id: ModuleId::AblativeShell,
        family: ModuleFamily::Ablative,
        recipe_fragments: 1,
        damage_basis_points: -2_000,
        cooldown_basis_points: 0,
        range_delta: -1,
        max_health_delta: 30,
    },
];

/// Enumerates the finite three-slot build space for balance and integrity checks.
#[must_use]
pub fn all_build_loadouts() -> Vec<Vec<ModuleId>> {
    (0_u16..1024)
        .filter(|mask| mask.count_ones() <= 3)
        .map(|mask| {
            ModuleId::ALL
                .into_iter()
                .enumerate()
                .filter_map(|(i, id)| ((mask & (1 << i)) != 0).then_some(id))
                .collect()
        })
        .collect()
}

pub(super) fn resolve(
    base: BaseCombatProfile,
    requested: &[ModuleId],
) -> Result<ResolvedBuild, ModuleError> {
    validate_catalog(&BUILD_MODULE_CATALOG)?;
    let modules = canonicalize_loadout(requested)?;
    let modifiers = aggregate_modifiers(&modules)?;
    let profile = EffectiveCombatProfile {
        damage: u32::try_from(scale_basis_points(
            u64::from(base.damage),
            modifiers.damage_basis_points,
        )?)
        .map_err(|_| ModuleError::ArithmeticOverflow)?,
        range: base
            .range
            .checked_add(modifiers.range_delta)
            .ok_or(ModuleError::ArithmeticOverflow)?,
        cooldown_ms: scale_basis_points(base.cooldown_ms, modifiers.cooldown_basis_points)?,
        max_health: i64::from(base.max_health)
            .checked_add(i64::from(modifiers.max_health_delta))
            .and_then(|v| u32::try_from(v).ok())
            .ok_or(ModuleError::ArithmeticOverflow)?,
    };
    if !(15..=100).contains(&profile.damage)
        || !(1..=15).contains(&profile.range)
        || !(60..=700).contains(&profile.cooldown_ms)
        || !(80..=150).contains(&profile.max_health)
    {
        return Err(ModuleError::EffectiveOutOfBounds("build profile"));
    }
    Ok(ResolvedBuild {
        catalog_revision: BUILD_CATALOG_REVISION,
        modules,
        modifiers,
        profile,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{dominates, resolve_build, ARSENAL_CATALOG_REVISION};
    #[test]
    fn builds_preserve_all_five_weapons_and_new_modules_have_costs_and_weaknesses() {
        let loadouts = all_build_loadouts();
        assert_eq!(loadouts.len(), 176);
        for (damage, range, cooldown_ms) in [
            (40, 6, 250),
            (25, 8, 150),
            (48, 9, 350),
            (56, 6, 400),
            (56, 10, 400),
        ] {
            let base = BaseCombatProfile {
                damage,
                range,
                cooldown_ms,
                max_health: 100,
            };
            for loadout in &loadouts {
                resolve(base, loadout).unwrap();
            }
            for module in [
                ModuleId::BreachShunt,
                ModuleId::StandoffOptic,
                ModuleId::SkirmishDrive,
                ModuleId::AblativeShell,
            ] {
                assert!(resolve_build(ARSENAL_CATALOG_REVISION, base, &[module]).is_err());
                let p = resolve(base, &[module]).unwrap().profile;
                assert!(!dominates(
                    p,
                    EffectiveCombatProfile {
                        damage,
                        range,
                        cooldown_ms,
                        max_health: 100
                    }
                ));
            }
        }
    }
}
