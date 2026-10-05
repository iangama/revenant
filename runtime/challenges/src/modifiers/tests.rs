use super::*;

#[test]
fn selection_is_bounded_contract_specific_and_order_independent() {
    use Modifier::{BastionFirst, Pace, SingleReserve, WestApproach};
    for (contract, pair, expected) in [
        (
            ContractId::MeridianCircuit,
            [WestApproach, Pace],
            Preset::WestApproachPace,
        ),
        (
            ContractId::RelayGauntlet,
            [BastionFirst, SingleReserve],
            Preset::BastionFirstSingleReserve,
        ),
        (
            ContractId::RelayGauntlet,
            [SingleReserve, Pace],
            Preset::SingleReservePace,
        ),
    ] {
        assert_eq!(Preset::select(contract, &pair), Ok(expected));
        assert_eq!(Preset::select(contract, &[pair[1], pair[0]]), Ok(expected));
    }
    for modifiers in [
        vec![BastionFirst, Pace],
        vec![SingleReserve, SingleReserve],
        vec![BastionFirst, SingleReserve, Pace],
        vec![WestApproach],
    ] {
        assert_eq!(
            Preset::select(ContractId::RelayGauntlet, &modifiers),
            Err(ChallengeError::InvalidModifiers)
        );
    }
    assert_eq!(
        Preset::select(ContractId::MeridianCircuit, &[SingleReserve]),
        Err(ChallengeError::InvalidModifiers)
    );
    for contract in ContractId::ALL {
        assert_eq!(Preset::select(contract, &[]), Ok(Preset::Baseline));
        if ![ContractId::MeridianCircuit, ContractId::RelayGauntlet].contains(&contract) {
            assert_eq!(
                Preset::select(contract, &[Pace]),
                Err(ChallengeError::InvalidModifiers)
            );
        }
    }
}

#[test]
fn west_approach_uses_retained_connected_walkable_geometry() {
    use std::collections::{BTreeSet, VecDeque};
    let sector: serde_json::Value =
        serde_json::from_str(include_str!("../../../../client/game/world/meridian.json")).unwrap();
    let walkable = |p: [i32; 3]| {
        sector["floors"].as_array().unwrap().iter().any(|r| {
            let c: Vec<i64> = r
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_i64().unwrap())
                .collect();
            (c[0]..=c[1]).contains(&i64::from(p[0])) && (c[2]..=c[3]).contains(&i64::from(p[2]))
        })
    };
    let west = Preset::WestApproach.meridian_entrance().unwrap();
    assert_ne!(Some(west), Preset::Baseline.meridian_entrance());
    assert!(walkable(west));
    let mut seen = BTreeSet::from([west]);
    let mut queue = VecDeque::from([west]);
    while let Some(p) = queue.pop_front() {
        for [dx, dz] in [[1, 0], [-1, 0], [0, 1], [0, -1]] {
            let next = [p[0] + dx, 0, p[2] + dz];
            if walkable(next) && seen.insert(next) {
                queue.push_back(next);
            }
        }
    }
    for objective in [
        crate::Objective::LensRead,
        crate::Objective::GalleryRead,
        crate::Objective::LogRead,
    ] {
        assert!(seen.contains(&objective.position().unwrap()));
    }
    assert_eq!(Preset::WestApproachPace.meridian_entrance(), Some(west));
    assert_eq!(Preset::SingleReserve.meridian_entrance(), None);
}
