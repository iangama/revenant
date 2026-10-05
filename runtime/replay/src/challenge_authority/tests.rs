use super::*;
use crate::{
    encode_challenge_payload, encode_module_state_snapshot, encode_support_evidence,
    module_state_evidence_for_catalog, reconstruct, ModuleStateSnapshotPayloadV1, SupportEvidence,
};
use revenant_challenges::{apply, Equipment, Outcome};
use revenant_modules::BUILD_CATALOG_REVISION;

#[path = "gauntlet_tests.rs"]
mod gauntlet;
#[path = "mastery_tests.rs"]
mod mastery;
#[path = "prism_tests.rs"]
mod prism;
#[path = "route_tests.rs"]
mod route;
#[path = "signal_tests.rs"]
mod signal;
#[path = "survival_tests.rs"]
mod survival;

fn event(events: &mut Vec<ReplayEvent>, kind: Kind, actor: u64, payload: String) -> i64 {
    let id = i64::try_from(events.len()).unwrap() + 1;
    events.push(ReplayEvent {
        id,
        kind,
        actor_id: Some(actor),
        payload,
        timestamp: id.to_string(),
        session_id: "challenge-session".into(),
        account_id: "local:challenge".into(),
        activity_id: events.first().and_then(|e| e.activity_id.clone()),
    });
    id
}

fn equipment() -> Equipment {
    Equipment {
        loadout_revision: 0,
        catalog_revision: BUILD_CATALOG_REVISION.into(),
        weapon_id: "pulse_rifle".into(),
        modules: vec![],
    }
}

fn checkpoint(
    events: &mut Vec<ReplayEvent>,
    before: &ChallengeState,
    command: Command,
    proof: Option<i64>,
) -> ChallengeState {
    let policy = [
        REVISION,
        revenant_challenges::EXPANDED_REVISION,
        revenant_challenges::ELITE_REVISION,
        revenant_challenges::PRISM_REVISION,
        revenant_challenges::SIGNAL_REVISION,
        revenant_challenges::SURVIVAL_REVISION,
        revenant_challenges::GAUNTLET_REVISION,
        revenant_challenges::MODIFIER_REVISION,
    ]
    .into_iter()
    .find(|policy| {
        before.supports_policy(policy)
            && (!matches!(command, Command::StartModified { .. })
                || *policy == revenant_challenges::MODIFIER_REVISION)
            && !matches!(command, Command::Start { contract, .. } if !contract.available_in(policy))
    })
    .unwrap();
    let transition = apply(policy, before, before.revision, &command).unwrap();
    let payload = ChallengeReplayPayload {
        schema_version: 1,
        policy_revision: policy.into(),
        character_id: "local:challenge:operator".into(),
        operation_id: format!("command-{}", before.revision),
        before: before.clone(),
        command,
        after: transition.state.clone(),
        first_completion: transition.first_completion,
        proof_event_id: proof,
    };
    event(
        events,
        Kind::ChallengeCheckpoint,
        1,
        encode_challenge_payload(&payload).unwrap(),
    );
    transition.state
}

fn admission(
    contract: ContractId,
    before: &ChallengeState,
    retry: bool,
) -> (Vec<ReplayEvent>, ChallengeState) {
    let mut events = vec![];
    event(&mut events, Kind::PlayerJoined, 1, "player joined".into());
    events[0].activity_id = Some(contract.activity_id().into());
    let snapshot = ModuleStateSnapshotPayloadV1 {
        schema_version: 1,
        character_id: "local:challenge:operator".into(),
        protocol_generation: ReplayProtocolGeneration::V2,
        state: module_state_evidence_for_catalog(
            BUILD_CATALOG_REVISION,
            0,
            &[],
            &[],
            0,
            ReplayProtocolGeneration::V2,
        )
        .unwrap(),
    };
    let proof = event(
        &mut events,
        Kind::ModuleStateSnapshot,
        1,
        encode_module_state_snapshot(&snapshot).unwrap(),
    );
    let command = if retry {
        Command::Retry {
            run_id: before.active.as_ref().unwrap().run_id,
            equipment: equipment(),
        }
    } else {
        Command::Start {
            contract,
            equipment: equipment(),
        }
    };
    let state = checkpoint(&mut events, before, command, Some(proof));
    (events, state)
}

fn visit(
    events: &mut Vec<ReplayEvent>,
    state: &ChallengeState,
    objective: Objective,
) -> ChallengeState {
    let command = Command::Confirm {
        run_id: state.active.as_ref().unwrap().run_id,
        objective,
    };
    let payload = ChallengeVisitProof::new(
        "local:challenge:operator",
        state,
        &command,
        objective.position().unwrap(),
    )
    .unwrap();
    let proof = event(
        events,
        Kind::ChallengeObjective,
        1,
        payload.encode().unwrap(),
    );
    checkpoint(events, state, command, Some(proof))
}

fn exploration() -> (Vec<ReplayEvent>, ChallengeState) {
    let (mut events, mut state) = admission(
        ContractId::MeridianCircuit,
        &ChallengeState::default(),
        false,
    );
    for objective in [
        Objective::GalleryRead,
        Objective::LogRead,
        Objective::LensRead,
        Objective::Returned,
    ] {
        state = visit(&mut events, &state, objective);
    }
    (events, state)
}

fn support(events: &mut Vec<ReplayEvent>, time: u64, transition: SupportTransition) -> i64 {
    let (kind, actor) = match transition {
        SupportTransition::Started { lancer_id, .. } => (Kind::EnemySpawned, lancer_id),
        SupportTransition::Attack {
            target_id,
            health_after: 0,
            ..
        } => (Kind::EnemyDied, target_id),
        _ => (Kind::FieldActivity, 1),
    };
    event(
        events,
        kind,
        actor,
        encode_support_evidence(&SupportEvidence {
            elapsed_ms: time,
            transition,
        })
        .unwrap(),
    )
}

fn combat() -> (Vec<ReplayEvent>, ChallengeState) {
    let (mut events, mut state) =
        admission(ContractId::CloseQuarters, &ChallengeState::default(), false);
    support(
        &mut events,
        0,
        SupportTransition::Started {
            lancer_id: 2,
            mender_id: 3,
            player_health: 100,
        },
    );
    let mut time = 0;
    for (target_id, objective, initial) in [
        (3, Objective::MenderCleared, 120u32),
        (2, Objective::LancerCleared, 200),
    ] {
        let mut health = initial;
        while health > 0 {
            let after = health.saturating_sub(40);
            let proof = support(
                &mut events,
                time,
                SupportTransition::Attack {
                    target_id,
                    player_position: [5, 0, 8],
                    damage: 40,
                    health_before: health,
                    health_after: after,
                },
            );
            health = after;
            time += 250;
            if health == 0 {
                state = checkpoint(
                    &mut events,
                    &state,
                    Command::Confirm {
                        run_id: 1,
                        objective,
                    },
                    Some(proof),
                );
            }
        }
    }
    (events, state)
}

#[test]
fn challenge_authority_reconstructs_both_contracts_without_rewards() {
    for (events, expected) in [exploration(), combat()] {
        let replay = reconstruct(&events).unwrap();
        assert_eq!(replay.challenge.unwrap().state, expected);
        assert_eq!(expected.last_result.unwrap().outcome, Outcome::Completed);
        assert_eq!(expected.records.len(), 1);
        assert_eq!((replay.loot_grants, replay.progression_grants), (0, 0));
        assert!(!replay.completed);
    }
}

#[test]
fn challenge_authority_rejects_foreign_admission_and_unaccepted_equipment() {
    for corruption in [
        "account",
        "actor",
        "character",
        "session",
        "activity",
        "v1",
        "loadout",
        "proof",
        "extra_player",
    ] {
        let (mut events, _) =
            admission(ContractId::CloseQuarters, &ChallengeState::default(), false);
        match corruption {
            "account" => events[0].account_id = "foreign".into(),
            "actor" => events[2].actor_id = Some(9),
            "character" => {
                events[1].payload = events[1]
                    .payload
                    .replace("local:challenge:operator", "foreign");
            }
            "session" => events[1].session_id = "foreign".into(),
            "activity" => events[0].activity_id = Some("relay_awakening".into()),
            "v1" => {
                let snapshot = ModuleStateSnapshotPayloadV1 {
                    schema_version: 1,
                    character_id: "local:challenge:operator".into(),
                    protocol_generation: ReplayProtocolGeneration::V1,
                    state: module_state_evidence_for_catalog(
                        BUILD_CATALOG_REVISION,
                        0,
                        &[],
                        &[],
                        0,
                        ReplayProtocolGeneration::V1,
                    )
                    .unwrap(),
                };
                events[1].payload = encode_module_state_snapshot(&snapshot).unwrap();
            }
            "loadout" => {
                events[2].payload = events[2]
                    .payload
                    .replace("\"loadout_revision\":0", "\"loadout_revision\":1");
            }
            "proof" => {
                events[2].payload = events[2]
                    .payload
                    .replace("\"proof_event_id\":2", "\"proof_event_id\":1");
            }
            _ => {
                event(&mut events, Kind::PlayerJoined, 7, "player joined".into());
            }
        }
        assert!(reconstruct(&events).is_err(), "{corruption}");
    }
}

#[test]
fn challenge_authority_rejects_forged_visits_and_orphan_proofs() {
    for corruption in [
        "actor",
        "character",
        "run",
        "revision",
        "position",
        "objective",
        "orphan",
        "duplicate",
        "reward",
    ] {
        let (mut events, _) = exploration();
        match corruption {
            "actor" => events[3].actor_id = Some(2),
            "character" => {
                events[3].payload = events[3]
                    .payload
                    .replace("local:challenge:operator", "foreign");
            }
            "run" => events[3].payload = events[3].payload.replace("\"run_id\":1", "\"run_id\":2"),
            "revision" => {
                events[3].payload = events[3]
                    .payload
                    .replace("\"state_revision\":1", "\"state_revision\":2");
            }
            "position" => events[3].payload = events[3].payload.replace("[-28,0,7]", "[-28,0,-7]"),
            "objective" => {
                events[3].payload = events[3].payload.replace("gallery_read", "lens_read");
            }
            "orphan" => events.truncate(4),
            "duplicate" => {
                events[6].payload = events[6]
                    .payload
                    .replace("\"proof_event_id\":6", "\"proof_event_id\":4");
            }
            _ => {
                event(&mut events, Kind::LootGranted, 1, "forged reward".into());
            }
        }
        assert!(reconstruct(&events).is_err(), "{corruption}");
    }
}

#[test]
fn challenge_authority_rejects_wrong_fatal_hit_and_post_terminal_activity() {
    for corruption in [
        "target",
        "actor",
        "nonfatal",
        "missing_spawn",
        "after_terminal",
        "unrelated",
    ] {
        let (mut events, _) = combat();
        match corruption {
            "target" => {
                events[7].payload = events[7]
                    .payload
                    .replace("mender_cleared", "lancer_cleared")
                    .replace("\"objectives\":2", "\"objectives\":1");
            }
            "actor" => events[6].actor_id = Some(2),
            "nonfatal" => {
                events[7].payload = events[7]
                    .payload
                    .replace("\"proof_event_id\":7", "\"proof_event_id\":6");
            }
            "missing_spawn" => {
                events.remove(3);
            }
            "after_terminal" => {
                support(&mut events, 3000, SupportTransition::Abandoned);
            }
            _ => {
                event(&mut events, Kind::CampaignObjective, 1, "foreign".into());
            }
        }
        assert!(reconstruct(&events).is_err(), "{corruption}");
    }
}

#[test]
fn challenge_authority_retry_restarts_world_and_rejects_old_attempt_evidence() {
    let (old, before) = admission(
        ContractId::MeridianCircuit,
        &ChallengeState::default(),
        false,
    );
    let (mut events, state) = admission(ContractId::MeridianCircuit, &before, true);
    assert_eq!(state.active.as_ref().unwrap().run_id, 2);
    assert_eq!(
        state.last_result.as_ref().unwrap().outcome,
        Outcome::Interrupted
    );
    assert_eq!(
        reconstruct(&events).unwrap().challenge.unwrap().state,
        state
    );
    let mut old_proof = ChallengeVisitProof::new(
        "local:challenge:operator",
        &before,
        &Command::Confirm {
            run_id: 1,
            objective: Objective::LensRead,
        },
        [-28, 0, -7],
    )
    .unwrap();
    let proof = event(
        &mut events,
        Kind::ChallengeObjective,
        1,
        old_proof.encode().unwrap(),
    );
    checkpoint(
        &mut events,
        &state,
        Command::Confirm {
            run_id: 2,
            objective: Objective::LensRead,
        },
        Some(proof),
    );
    assert!(reconstruct(&events).is_err());
    old_proof.run_id = 2;
    old_proof.state_revision = state.revision;
    events[3].payload = old_proof.encode().unwrap();
    assert!(reconstruct(&events).is_ok());
    let mut same_session = old;
    checkpoint(
        &mut same_session,
        &before,
        Command::Retry {
            run_id: 1,
            equipment: equipment(),
        },
        Some(2),
    );
    assert!(reconstruct(&same_session).is_err());
}

#[test]
fn challenge_authority_checks_damage_range_cooldown_and_starting_health() {
    for corruption in ["damage", "range", "cooldown", "health"] {
        let (mut events, _) = combat();
        events.truncate(6);
        match corruption {
            "damage" => {
                events.truncate(5);
                events[4].payload = events[4]
                    .payload
                    .replace("\"damage\":40", "\"damage\":80")
                    .replace("\"health_after\":80", "\"health_after\":40");
            }
            "range" => events[4].payload = events[4].payload.replace("[5,0,8]", "[2,0,10]"),
            "cooldown" => {
                events[5].payload = events[5]
                    .payload
                    .replace("\"elapsed_ms\":250", "\"elapsed_ms\":1");
            }
            _ => {
                events[3].payload = events[3]
                    .payload
                    .replace("\"player_health\":100", "\"player_health\":90");
            }
        }
        assert!(reconstruct(&events).is_err(), "{corruption}");
    }
}

#[test]
fn challenge_authority_defeat_requires_a_lethal_charge() {
    let (mut events, state) =
        admission(ContractId::CloseQuarters, &ChallengeState::default(), false);
    support(
        &mut events,
        0,
        SupportTransition::Started {
            lancer_id: 2,
            mender_id: 3,
            player_health: 100,
        },
    );
    let mut health = 100u32;
    let mut origin = revenant_ai::lancer::SPAWN;
    let mut time = 700;
    let mut proof = 0;
    while health > 0 {
        support(
            &mut events,
            time,
            SupportTransition::Windup {
                player_position: [5, 0, 8],
                origin,
                target: [5, 0, 8],
            },
        );
        health = health.saturating_sub(18);
        proof = support(
            &mut events,
            time + 1800,
            SupportTransition::Charged {
                player_position: [5, 0, 8],
                origin,
                target: [5, 0, 8],
                damage: 18,
                health_after: health,
            },
        );
        origin = [5, 0, 8];
        time += 3200;
    }
    checkpoint(
        &mut events,
        &state,
        Command::End {
            run_id: 1,
            reason: EndReason::Defeated,
        },
        Some(proof),
    );
    assert_eq!(
        reconstruct(&events)
            .unwrap()
            .challenge
            .unwrap()
            .state
            .last_result
            .unwrap()
            .outcome,
        Outcome::Defeated
    );
    let last = events.last_mut().unwrap();
    let mut payload = decode_challenge_payload(&last.payload).unwrap();
    payload.proof_event_id = Some(proof - 1);
    last.payload = encode_challenge_payload(&payload).unwrap();
    assert!(reconstruct(&events).is_err());
}

struct RecoveryFixture {
    events: Vec<ReplayEvent>,
    challenge: ChallengeState,
    run: revenant_challenges::recovery::RecoveryRun,
    sequence: u64,
    time: u64,
}

impl RecoveryFixture {
    fn new() -> Self {
        let (events, challenge) = admission(
            ContractId::CoolantRecovery,
            &ChallengeState::default(),
            false,
        );
        Self {
            events,
            challenge,
            run: revenant_challenges::recovery::RecoveryRun::default(),
            sequence: 0,
            time: 0,
        }
    }

    fn step(&mut self, elapsed_ms: u64, action: revenant_challenges::recovery::Action) {
        let next = self.run.apply(elapsed_ms, action).unwrap();
        self.sequence += 1;
        self.time = elapsed_ms;
        let evidence = crate::RecoveryEvidence {
            run_id: self.challenge.active.as_ref().unwrap().run_id,
            sequence: self.sequence,
            elapsed_ms,
            action,
            checkpoint: next.checkpoint,
        };
        let proof = event(
            &mut self.events,
            Kind::FieldActivity,
            1,
            evidence.encode().unwrap(),
        );
        if let Some(objective) = evidence.objective() {
            self.challenge = checkpoint(
                &mut self.events,
                &self.challenge,
                Command::Confirm {
                    run_id: evidence.run_id,
                    objective,
                },
                Some(proof),
            );
        }
        self.run = next.run;
    }

    fn walk(&mut self, target: [i32; 3]) {
        while self.run.position() != target {
            let mut position = self.run.position();
            let axis = if position[0] == target[0] { 2 } else { 0 };
            position[axis] += (target[axis] - position[axis]).signum();
            self.step(
                self.time + 100,
                revenant_challenges::recovery::Action::Move { position },
            );
        }
    }
}

#[test]
fn recovery_replay_binds_ordered_holds_to_the_exact_attempt_actor_and_move_stream() {
    use revenant_challenges::recovery::{Action, Checkpoint, DELIVERY, DWELL_MS, INTAKE, TRANSFER};
    let mut fixture = RecoveryFixture::new();
    fixture.walk(INTAKE);
    fixture.walk(TRANSFER);
    fixture.step(fixture.time + DWELL_MS - 1, Action::Observe);
    fixture.walk([3, 0, -5]);
    fixture.walk(TRANSFER);
    fixture.step(fixture.time + DWELL_MS, Action::Observe);
    fixture.walk(DELIVERY);
    fixture.step(fixture.time + DWELL_MS, Action::Observe);
    let result = reconstruct(&fixture.events).unwrap().challenge.unwrap();
    assert_eq!(result.state, fixture.challenge);
    assert_eq!(
        result.state.last_result.unwrap().outcome,
        Outcome::Completed
    );
    assert_eq!(result.state.records.len(), 1);
    let held = fixture
        .events
        .iter()
        .position(|event| {
            crate::RecoveryEvidence::decode(&event.payload)
                .is_ok_and(|proof| proof.checkpoint == Some(Checkpoint::Transferred))
        })
        .unwrap();
    for change in 0..5 {
        let mut forged = fixture.events.clone();
        let mut proof = crate::RecoveryEvidence::decode(&forged[held].payload).unwrap();
        match change {
            0 => proof.elapsed_ms -= 1,
            1 => proof.run_id += 1,
            2 => proof.sequence += 1,
            3 => forged[held].actor_id = Some(99),
            _ => proof.action = Action::Move { position: TRANSFER },
        }
        forged[held].payload = proof.encode().unwrap();
        assert!(
            reconstruct(&forged).is_err(),
            "accepted forged recovery variant {change}"
        );
    }
    let mut missing = fixture.events.clone();
    missing.remove(held - 1);
    assert!(reconstruct(&missing).is_err());
    let mut unpaired = fixture.events;
    unpaired.remove(held + 1);
    assert!(reconstruct(&unpaired).is_err());
}

#[test]
fn recovery_replay_rejects_instant_station_claims_and_other_contract_evidence() {
    let mut fixture = RecoveryFixture::new();
    fixture.walk(revenant_challenges::recovery::INTAKE);
    let command = Command::Confirm {
        run_id: 1,
        objective: Objective::CoolantTransferred,
    };
    assert!(ChallengeVisitProof::new(
        "local:challenge:operator",
        &fixture.challenge,
        &command,
        revenant_challenges::recovery::TRANSFER
    )
    .is_err());
    let orphan = fixture
        .events
        .iter()
        .find(|event| event.payload.starts_with(crate::challenge_recovery::PREFIX))
        .unwrap()
        .clone();
    assert!(reconstruct(&[orphan]).is_err());
    let mut wrong_contract = fixture.events;
    for event in &mut wrong_contract {
        event.activity_id = Some(ContractId::MeridianCircuit.activity_id().into());
    }
    assert!(reconstruct(&wrong_contract).is_err());
}

fn elite_combat() -> (Vec<ReplayEvent>, ChallengeState) {
    elite_combat_order(false)
}

fn elite_combat_order(bulwark_first: bool) -> (Vec<ReplayEvent>, ChallengeState) {
    use crate::{encode_elite_evidence, EliteEvidence, EliteTransition};
    let (mut events, mut state) =
        admission(ContractId::BastionLink, &ChallengeState::default(), false);
    let encode = |time, previous, transition| {
        encode_elite_evidence(&EliteEvidence {
            elapsed_ms: time,
            previous_confirmed_ms: previous,
            transition,
        })
        .unwrap()
    };
    event(
        &mut events,
        Kind::EnemySpawned,
        2,
        encode(
            0,
            0,
            EliteTransition::Started {
                composition: "bastion_link".into(),
                bulwark_id: 2,
                partner_id: 3,
                player_health: 100,
            },
        ),
    );
    let mut time: u64 = 0;
    let mut targets = [
        (3, Objective::MenderCleared, 120u32),
        (2, Objective::BulwarkCleared, 240),
    ];
    if bulwark_first {
        targets.reverse();
    }
    for (target_id, objective, initial) in targets {
        let mut health = initial;
        while health > 0 {
            let after = health.saturating_sub(40);
            let proof = event(
                &mut events,
                if after == 0 {
                    Kind::EnemyDied
                } else {
                    Kind::FieldActivity
                },
                if after == 0 { target_id } else { 1 },
                encode(
                    time,
                    time.saturating_sub(250),
                    EliteTransition::Attack {
                        target_id,
                        player_position: [10, 0, -8],
                        damage: 40,
                        health_before: health,
                        health_after: after,
                    },
                ),
            );
            health = after;
            time += 250;
            if health == 0 {
                state = checkpoint(
                    &mut events,
                    &state,
                    Command::Confirm {
                        run_id: 1,
                        objective,
                    },
                    Some(proof),
                );
            }
        }
    }
    (events, state)
}

#[test]
fn challenge_elite_binds_bastion_completion_to_the_build_without_rewards() {
    let (events, expected) = elite_combat();
    let replay = reconstruct(&events).unwrap();
    assert_eq!(replay.challenge.unwrap().state, expected);
    assert_eq!((replay.loot_grants, replay.progression_grants), (0, 0));
    assert!(!replay.completed);
    assert_eq!(expected.records[0].rules.seed, 37_004);
    let mut wrong_weapon = events[..5].to_vec();
    wrong_weapon[2].payload = wrong_weapon[2]
        .payload
        .replace("pulse_rifle", "arc_sidearm");
    assert!(reconstruct(&wrong_weapon).is_err());
    let mut wrong_composition = events[..4].to_vec();
    wrong_composition[3].payload = wrong_composition[3]
        .payload
        .replace("bastion_link", "crossed_guard");
    assert!(reconstruct(&wrong_composition).is_err());
}

#[test]
fn challenge_elite_rejects_uncommitted_fatal_hits_and_cooldown_bypass() {
    let (events, _) = elite_combat();
    let death = events
        .iter()
        .position(|e| e.kind == Kind::EnemyDied)
        .unwrap();
    assert!(reconstruct(&events[..=death]).is_err());
    let mut too_fast = events[..6].to_vec();
    too_fast[5].payload = too_fast[5]
        .payload
        .replace("\"elapsed_ms\":250", "\"elapsed_ms\":249");
    assert!(reconstruct(&too_fast).is_err());
    let mut wrong_target = events.clone();
    let index = wrong_target
        .iter()
        .position(|e| e.kind == Kind::ChallengeCheckpoint && e.payload.contains("mender_cleared"))
        .unwrap();
    // Change both command/result consistently: the domain can accept it, but
    // the cited world death must still identify the actual killed target.
    wrong_target[index].payload = wrong_target[index]
        .payload
        .replace("mender_cleared", "bulwark_cleared")
        .replace("\"objectives\":2", "\"objectives\":1");
    assert!(reconstruct(&wrong_target[..=index]).is_err());
}

#[test]
fn challenge_modified_meridian_rejects_legacy_visit_streams() {
    use revenant_challenges::{modifiers::Preset, MODIFIER_REVISION};
    let contract = ContractId::MeridianCircuit;
    let (events, _) = admission(contract, &ChallengeState::default(), false);
    let base = decode_challenge_payload(&events.last().unwrap().payload).unwrap();
    let start = Command::StartModified {
        contract,
        preset: Preset::WestApproach,
        equipment: equipment(),
    };
    let modified = apply(MODIFIER_REVISION, &ChallengeState::default(), 0, &start)
        .unwrap()
        .state;
    for (before, command) in [
        (ChallengeState::default(), start),
        (
            modified,
            Command::Retry {
                run_id: 1,
                equipment: equipment(),
            },
        ),
    ] {
        let after = apply(MODIFIER_REVISION, &before, before.revision, &command)
            .unwrap()
            .state;
        let payload = ChallengeReplayPayload {
            policy_revision: MODIFIER_REVISION.into(),
            before,
            command,
            after,
            ..base.clone()
        };
        let mut attempt = events.clone();
        attempt.last_mut().unwrap().payload = encode_challenge_payload(&payload).unwrap();
        assert!(reconstruct(&attempt).is_ok());
        let state = payload.after;
        let command = Command::Confirm {
            run_id: state.active.as_ref().unwrap().run_id,
            objective: Objective::LensRead,
        };
        let legacy = ChallengeVisitProof::for_policy(
            MODIFIER_REVISION,
            "local:challenge:operator",
            &state,
            &command,
            [-28, 0, -7],
        )
        .unwrap();
        let proof = event(
            &mut attempt,
            Kind::ChallengeObjective,
            1,
            legacy.encode().unwrap(),
        );
        checkpoint(&mut attempt, &state, command, Some(proof));
        assert!(reconstruct(&attempt).is_err());
    }
}

#[test]
fn challenge_modifier_visits_follow_the_selected_return_position() {
    use revenant_challenges::{modifiers::Preset, MODIFIER_REVISION};
    let command = Command::StartModified {
        contract: ContractId::MeridianCircuit,
        preset: Preset::WestApproach,
        equipment: equipment(),
    };
    let mut state = apply(MODIFIER_REVISION, &ChallengeState::default(), 0, &command)
        .unwrap()
        .state;
    for objective in [
        Objective::LensRead,
        Objective::GalleryRead,
        Objective::LogRead,
    ] {
        let command = Command::Confirm {
            run_id: 1,
            objective,
        };
        assert!(ChallengeVisitProof::for_policy(
            MODIFIER_REVISION,
            "operator",
            &state,
            &command,
            objective.position().unwrap()
        )
        .is_ok());
        state = apply(MODIFIER_REVISION, &state, state.revision, &command)
            .unwrap()
            .state;
    }
    let command = Command::Confirm {
        run_id: 1,
        objective: Objective::Returned,
    };
    assert!(ChallengeVisitProof::for_policy(
        MODIFIER_REVISION,
        "operator",
        &state,
        &command,
        [-31, 0, 0]
    )
    .is_ok());
    assert!(ChallengeVisitProof::for_policy(
        MODIFIER_REVISION,
        "operator",
        &state,
        &command,
        [-16, 0, 0]
    )
    .is_err());
    assert!(ChallengeVisitProof::for_policy(
        revenant_challenges::GAUNTLET_REVISION,
        "operator",
        &state,
        &command,
        [-31, 0, 0]
    )
    .is_err());
}
