use super::*;
use crate::{arsenal_module_snapshot, resolved_arsenal_projection};
use revenant_inventory::{RAIL_DRIVER, SCATTER_CASTER};

const PLAYER: u64 = 5035;

#[test]
fn saved_build_admission_keeps_v1_base_and_rejects_unsupported_v2_loadout() {
    let modules = [ModuleId::StandoffOptic, ModuleId::SkirmishDrive];
    for (generation, build_capable, accepted, health, damage) in [
        (ProtocolGeneration::CurrentV2, false, false, 100, 40),
        (ProtocolGeneration::CurrentV2, true, true, 80, 36),
        (ProtocolGeneration::FrozenV1, false, true, 100, 40),
    ] {
        let (mut participant, _receiver) =
            fixture_participant("local:build-admission", PLAYER, &["pulse_rifle"]);
        participant.protocol_generation = generation;
        participant.content_capable = generation == ProtocolGeneration::CurrentV2;
        participant.arsenal_capable = participant.content_capable;
        participant.build_capable = build_capable;
        let persistence =
            FixturePersistence::new(None, Arc::new(Mutex::new(Vec::new())), Vec::new())
                .with_module_state(0, &modules, &modules, 1);
        let mut session = fixture_session(1, Vec::new(), SessionStage::Waiting, persistence);
        assert_eq!(session.join(participant).is_ok(), accepted);
        if accepted {
            let joined = &session.participants[0];
            assert_eq!(joined.module_state.loadout, modules);
            assert_eq!(joined.actor.health, health);
            assert_eq!(joined.admitted_module_weapons[0].effective_damage, damage);
        } else {
            assert!(session.participants.is_empty());
        }
    }
}

#[test]
fn build_previews_are_authoritative_and_require_new_catalog() {
    let (mut session, receiver) = setup(true);
    let modules = vec![
        "module_focus_lens".to_owned(),
        "module_breach_shunt".to_owned(),
    ];
    session
        .preview_modules(
            PLAYER,
            &ModulePreviewRequest {
                modules: modules.clone(),
            },
        )
        .unwrap();
    assert!(matches!(receiver.recv().unwrap(), ServerMessage::ModulePreview(p) if !p.accepted));
    session.participants[0].build_capable = true;
    let before = session.participants[0].admitted_module_weapons.clone();
    for (modules, weapon, damage, range, cooldown, health) in [
        (modules, "scatter_caster", 84, 3, 540, 100),
        (
            vec![
                "module_standoff_optic".to_owned(),
                "module_skirmish_drive".to_owned(),
            ],
            "rail_driver",
            50,
            13,
            380,
            80,
        ),
        (
            vec![
                "module_cycle_bypass".to_owned(),
                "module_ablative_shell".to_owned(),
            ],
            "pulse_rifle",
            32,
            3,
            200,
            130,
        ),
    ] {
        session
            .preview_modules(PLAYER, &ModulePreviewRequest { modules })
            .unwrap();
        let ServerMessage::ModulePreview(preview) = receiver.recv().unwrap() else {
            panic!("preview expected")
        };
        assert!(preview.accepted);
        let profile = preview
            .weapons
            .iter()
            .find(|p| p.item_id == weapon)
            .unwrap();
        assert_eq!(
            (
                profile.effective_damage,
                profile.effective_range,
                profile.effective_cooldown_ms,
                preview.max_health
            ),
            (damage, range, cooldown, health)
        );
        assert_eq!(session.participants[0].admitted_module_weapons, before);
        assert!(session.participants[0].module_state.loadout.is_empty());
    }
    let state = &session.participants[0].module_state;
    assert_eq!(
        arsenal_module_snapshot(state, true, Some("m35-v1"))
            .unwrap()
            .catalog
            .len(),
        6
    );
    assert_eq!(
        arsenal_module_snapshot(state, true, Some("m35-v2"))
            .unwrap()
            .catalog
            .len(),
        10
    );
}

fn setup(capable: bool) -> (SharedSession, Receiver<ServerMessage>) {
    let (mut player, receiver) = fixture_participant(
        "local:arsenal",
        PLAYER,
        &["pulse_rifle", SCATTER_CASTER, RAIL_DRIVER],
    );
    player.content_capable = true;
    player.arsenal_capable = capable;
    player.admitted_module_weapons = resolved_arsenal_projection(
        &player.module_state,
        true,
        true,
        capable.then_some("m35-v1"),
    )
    .unwrap()
    .weapons;
    let persistence = FixturePersistence::new(None, Arc::new(Mutex::new(Vec::new())), Vec::new());
    let mut session = fixture_session(1, vec![player], SessionStage::Waiting, persistence);
    session.start_if_ready(PLAYER).unwrap();
    drain_messages(&receiver);
    (session, receiver)
}

#[test]
fn arsenal_scatter_falloff_shared_cooldown_and_rail_dead_zone() {
    let (mut session, receiver) = setup(true);
    let target = session.enemy_id.unwrap();
    let mut enemy = session.actors.get(target).unwrap().clone();
    enemy.health = 500;
    enemy.max_health = 500;
    enemy.position = [4, 0, 0];
    session.actors.insert(enemy);
    session.equip_weapon(PLAYER, SCATTER_CASTER).unwrap();
    session.attack_at(PLAYER, target, 0).unwrap();
    assert_eq!(session.actors.get(target).unwrap().health, 472);
    session.equip_weapon(PLAYER, RAIL_DRIVER).unwrap();
    assert!(session.attack_at(PLAYER, target, 399).is_err());
    session.attack_at(PLAYER, target, 400).unwrap();
    assert_eq!(session.actors.get(target).unwrap().health, 416);
    session.actors.update_position(target, [3, 0, 0]);
    assert!(session.attack_at(PLAYER, target, 800).is_err());
    // A rejected dead-zone shot consumes no server cooldown.
    session.equip_weapon(PLAYER, SCATTER_CASTER).unwrap();
    session.attack_at(PLAYER, target, 800).unwrap();
    assert_eq!(session.actors.get(target).unwrap().health, 360);
    session.actors.update_position(target, [7, 0, 0]);
    assert!(session.attack_at(PLAYER, target, 1200).is_err());
    assert_eq!(
        drain_messages(&receiver)
            .iter()
            .filter(|m| matches!(m, ServerMessage::DamageApplied(d) if d.source_actor_id == PLAYER))
            .count(),
        3
    );
}

#[test]
fn arsenal_catalog_is_opt_in_and_matches_replay_profiles() {
    let (mut legacy, receiver) = setup(false);
    for weapon in [SCATTER_CASTER, RAIL_DRIVER] {
        legacy.equip_weapon(PLAYER, weapon).unwrap();
        assert_eq!(
            legacy.participants[0].equipped_weapon_item_id,
            "pulse_rifle"
        );
    }
    assert_eq!(
        drain_messages(&receiver)
            .iter()
            .filter(|m| matches!(m, ServerMessage::EquipmentChanged(e) if !e.accepted))
            .count(),
        2
    );
    let state = &legacy.participants[0].module_state;
    let snapshot = arsenal_module_snapshot(state, true, Some("m35-v1")).unwrap();
    assert_eq!(snapshot.catalog_revision, "m35-v1");
    assert_eq!(snapshot.weapons.len(), 5);
    let replay = revenant_replay::module_state_evidence_for_catalog(
        "m35-v1",
        0,
        &[],
        &[],
        0,
        revenant_replay::ReplayProtocolGeneration::V2,
    )
    .unwrap();
    for (wire, saved) in snapshot.weapons.iter().zip(replay.weapons) {
        assert_eq!(wire.item_id, saved.item_id);
        assert_eq!(wire.effective_damage, saved.effective.damage);
        assert_eq!(wire.effective_range, saved.effective.range);
        assert_eq!(wire.effective_cooldown_ms, saved.effective.cooldown_ms);
    }
    assert_eq!(
        arsenal_module_snapshot(state, true, None)
            .unwrap()
            .weapons
            .len(),
        3
    );
    assert_eq!(
        arsenal_module_snapshot(state, false, None)
            .unwrap()
            .weapons
            .len(),
        2
    );
}

#[test]
fn route_evidence_preserves_negotiated_arsenal_catalog_with_empty_loadout() {
    let (mut player, _) = fixture_participant("local:route-catalog", PLAYER, &["pulse_rifle"]);
    player.arsenal_capable = true;
    player.module_state.owned_modules = vec![ModuleId::BreachShunt];
    for (build, revision, owned) in [(false, "m35-v1", 0), (true, "m35-v2", 1)] {
        player.build_capable = build;
        let evidence = player.replay_module_state().unwrap();
        assert_eq!(evidence.catalog_revision, revision);
        assert_eq!(evidence.weapons.len(), 5);
        assert_eq!(evidence.owned_modules.len(), owned);
    }
}
