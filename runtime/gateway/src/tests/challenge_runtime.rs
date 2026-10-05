use std::{
    collections::{BTreeMap, VecDeque},
    env,
    sync::mpsc::Receiver,
    time::{SystemTime, UNIX_EPOCH},
};

use revenant_challenges::{Command, ContractId, EndReason, Outcome};
use revenant_persistence::{ChallengeRequest, ModuleMutationReplayContext, Persistence};
use revenant_protocol::{ModuleCombineIntent, ModuleLoadoutIntent, ServerMessage};

use crate::{challenges::Entry, tests::fixture_participant, SessionStage, SharedSession};

struct Fixture {
    url: String,
    account: String,
    character: String,
}

impl Fixture {
    fn new() -> Option<Self> {
        let Ok(url) = env::var("DATABASE_URL") else {
            eprintln!("DATABASE_URL is not set; challenge runtime PostgreSQL test skipped");
            return None;
        };
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let account = format!("local:m37-world-{id}");
        let mut db = Persistence::connect(&url).unwrap();
        db.ensure_local_account(&account, "Challenge world")
            .unwrap();
        Some(Self {
            url,
            character: format!("{account}:operator"),
            account,
        })
    }

    fn enter(
        &self,
        contract: ContractId,
        retry: Option<u64>,
    ) -> (SharedSession, Receiver<ServerMessage>) {
        self.enter_preset(
            contract,
            retry,
            revenant_challenges::modifiers::Preset::Baseline,
        )
    }

    fn enter_preset(
        &self,
        contract: ContractId,
        retry: Option<u64>,
        preset: revenant_challenges::modifiers::Preset,
    ) -> (SharedSession, Receiver<ServerMessage>) {
        // These deterministic world drivers use actor 41. Reserve that fixture
        // identity before spawning enemies from the shared process allocator.
        while revenant_actors::allocate_actor_id() <= 41 {}
        let mut db = Persistence::connect_existing(&self.url).unwrap();
        let state = db
            .challenge_state_for(&self.account, &self.character)
            .unwrap();
        let mut session =
            SharedSession::new(&self.url, "../../scripts/activities/relay_awakening.lua", 1)
                .unwrap();
        let (mut player, messages) =
            fixture_participant(&self.account, 41, &["pulse_rifle", "arc_sidearm"]);
        player.content_capable = true;
        player.build_capable = true;
        player.arsenal_capable = true;
        player.support_capable = true;
        player.elite_capable = true;
        player.prism_capable = true;
        player.exploration_capable = true;
        player.acquisition_capable = true;
        player.challenge_entry = Some(Entry {
            preset,
            operation_id: format!("entry-{}", state.revision),
            policy: [
                revenant_challenges::REVISION,
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
                state.supports_policy(policy)
                    && contract.available_in(policy)
                    && (preset.is_baseline() || *policy == revenant_challenges::MODIFIER_REVISION)
            })
            .unwrap(),
            expected_revision: state.revision,
            contract,
            retry_run_id: retry,
        });
        session.join(player).unwrap();
        session.start_if_ready(41).unwrap();
        (session, messages)
    }

    fn state(&self) -> revenant_challenges::ChallengeState {
        Persistence::connect_existing(&self.url)
            .unwrap()
            .challenge_state_for(&self.account, &self.character)
            .unwrap()
    }

    fn end_externally(&self, session: &SharedSession) {
        let state = self.state();
        Persistence::connect_existing(&self.url)
            .unwrap()
            .apply_challenge_command(
                &self.character,
                &ChallengeRequest {
                    operation_id: "external-abandon",
                    expected_revision: state.revision,
                    command: Command::End {
                        run_id: state.active.unwrap().run_id,
                        reason: EndReason::Abandoned,
                    },
                    proof_event_id: None,
                    position: None,
                },
                ModuleMutationReplayContext {
                    catalog_revision: Some("m35-v2"),
                    session_id: &session.session_id,
                    account_id: &self.account,
                    activity_id: session.challenge_activity_id().unwrap(),
                    actor_id: 41,
                },
            )
            .unwrap();
    }
}

fn walk(session: &mut SharedSession, messages: &Receiver<ServerMessage>, target: [i32; 3]) {
    let start = session.actors.get(41).unwrap().position;
    let mut frontier = VecDeque::from([start]);
    let mut previous = BTreeMap::from([(start, start)]);
    while let Some(position) = frontier.pop_front() {
        if position == target {
            break;
        }
        for offset in [[1, 0, 0], [-1, 0, 0], [0, 0, 1], [0, 0, -1]] {
            let next = [position[0] + offset[0], 0, position[2] + offset[2]];
            if !previous.contains_key(&next)
                && revenant_activities::meridian::walkable(next)
                && revenant_activities::meridian::valid_step(position, next)
            {
                previous.insert(next, position);
                frontier.push_back(next);
            }
        }
    }
    let mut steps = vec![];
    let mut point = target;
    while point != start {
        steps.push(point);
        point = previous[&point];
    }
    for position in steps.into_iter().rev() {
        session.move_player(41, position).unwrap();
        assert_eq!(session.actors.get(41).unwrap().position, position);
        for message in messages.try_iter() {
            assert!(!matches!(
                message,
                ServerMessage::LootGranted(_) | ServerMessage::ProgressionGranted(_)
            ));
        }
    }
}

#[test]
fn challenge_world_meridian_visits_real_actors_and_disconnect_restarts_a_whole_attempt() {
    let Some(f) = Fixture::new() else {
        return;
    };
    let (mut session, messages) = f.enter(ContractId::MeridianCircuit, None);
    session.move_player(41, [-28, 0, -7]).unwrap();
    assert_eq!(session.actors.get(41).unwrap().position, [-16, 0, 0]);
    assert_eq!(f.state().active.unwrap().objectives, 0);
    walk(&mut session, &messages, [-28, 0, -7]);
    assert_eq!(f.state().active.unwrap().objectives, 1);
    session.disconnect(41).unwrap();
    assert_eq!(
        f.state().last_result.as_ref().unwrap().outcome,
        Outcome::Interrupted
    );
    let (mut session, messages) = f.enter(ContractId::MeridianCircuit, Some(1));
    assert_eq!(f.state().active.unwrap().objectives, 0);
    for position in [[-28, 0, 7], [-30, 0, 0], [-28, 0, -7], [-16, 0, 0]] {
        walk(&mut session, &messages, position);
    }
    assert_eq!(session.stage, SessionStage::Complete);
    assert_eq!(f.state().records.len(), 1);
    let mut db = Persistence::connect_existing(&f.url).unwrap();
    assert_eq!(
        db.progression_for(&f.character)
            .unwrap()
            .unwrap()
            .experience,
        0
    );
    assert_eq!(
        db.campaign_state_for(&f.account, &f.character).unwrap(),
        revenant_campaign::CampaignState::default()
    );
}

#[test]
fn challenge_world_combat_uses_atomic_deaths_and_blocks_other_activities_and_loadout_changes() {
    let Some(f) = Fixture::new() else {
        return;
    };
    let (mut session, messages) = f.enter(ContractId::CloseQuarters, None);
    let original_weapon = session.participants[0].equipped_weapon_item_id.clone();
    session.equip_weapon(41, "arc_sidearm").unwrap();
    assert_eq!(
        session.participants[0].equipped_weapon_item_id,
        original_weapon
    );
    assert!(session.request_route_state(41).is_err());
    assert!(session.request_cooperation_state(41).is_err());
    assert!(session.send_acquisition_state(41).is_err());
    assert!(session
        .set_module_loadout(
            41,
            &ModuleLoadoutIntent {
                operation_id: "loadout".into(),
                expected_revision: 0,
                modules: vec![]
            }
        )
        .is_err());
    assert!(session
        .combine_module(
            41,
            &ModuleCombineIntent {
                operation_id: "combine".into(),
                module_id: "module_impact_core".into()
            }
        )
        .is_err());
    for position in [[3, 0, 6], [4, 0, 6], [5, 0, 6], [5, 0, 7], [5, 0, 8]] {
        session.move_player(41, position).unwrap();
    }
    let pair = session.support.as_ref().unwrap();
    let targets = [pair.mender_id, pair.lancer_id];
    let mut time = 0;
    for target in targets {
        while session.actors.get(target).is_some() {
            session.attack_at(41, target, time).unwrap();
            time += 250;
            for message in messages.try_iter() {
                assert!(!matches!(
                    message,
                    ServerMessage::LootGranted(_) | ServerMessage::ProgressionGranted(_)
                ));
            }
        }
    }
    assert_eq!(session.stage, SessionStage::Complete);
    assert_eq!(f.state().records.len(), 1);
    assert_eq!(f.state().last_result.unwrap().outcome, Outcome::Completed);
}

#[test]
fn challenge_world_failed_persistence_withholds_fatal_hit_and_exploration_movement() {
    let Some(f) = Fixture::new() else {
        return;
    };
    let (mut session, messages) = f.enter(ContractId::MeridianCircuit, None);
    walk(&mut session, &messages, [-27, 0, -7]);
    f.end_externally(&session);
    assert!(session.move_player(41, [-28, 0, -7]).is_err());
    assert_eq!(session.actors.get(41).unwrap().position, [-27, 0, -7]);
    assert!(messages.try_recv().is_err());
    assert!(f.state().records.is_empty());

    let Some(f) = Fixture::new() else {
        return;
    };
    let (mut session, messages) = f.enter(ContractId::CloseQuarters, None);
    for position in [[3, 0, 6], [4, 0, 6], [5, 0, 6], [5, 0, 7], [5, 0, 8]] {
        session.move_player(41, position).unwrap();
    }
    let target = session.support.as_ref().unwrap().mender_id;
    session.attack_at(41, target, 0).unwrap();
    session.attack_at(41, target, 250).unwrap();
    for _ in messages.try_iter() {}
    f.end_externally(&session);
    assert!(session.attack_at(41, target, 500).is_err());
    assert_eq!(session.actors.get(target).unwrap().health, 40);
    assert!(messages.try_recv().is_err());
    assert!(f.state().records.is_empty());
}

#[test]
fn challenge_world_lethal_charge_persists_defeat_before_projection() {
    let Some(f) = Fixture::new() else {
        return;
    };
    let (mut session, messages) = f.enter(ContractId::CloseQuarters, None);
    for position in [[3, 0, 6], [4, 0, 6], [5, 0, 6], [5, 0, 7], [5, 0, 8]] {
        session.move_player(41, position).unwrap();
    }
    let mut time = 700;
    let mut saw_defeat = false;
    for _ in messages.try_iter() {}
    while session.actors.get(41).unwrap().health > 0 {
        session.tick_support_at(time).unwrap();
        session.tick_support_at(time + 1800).unwrap();
        saw_defeat |= messages.try_iter().any(|m| matches!(m,
            ServerMessage::ChallengeSnapshot(s) if s.last_result.as_ref().is_some_and(|r|r.outcome=="defeated")));
        time += 3200;
    }
    assert_eq!(session.stage, SessionStage::Failed);
    let state = f.state();
    assert_eq!(
        state.last_result.as_ref().unwrap().outcome,
        Outcome::Defeated
    );
    assert!(state.records.is_empty());
    assert!(saw_defeat);
}

fn recovery_walk(
    session: &mut SharedSession,
    messages: &Receiver<ServerMessage>,
    target: [i32; 3],
    time: &mut u64,
) {
    use revenant_challenges::recovery::Action;
    while session.actors.get(41).unwrap().position != target {
        let mut position = session.actors.get(41).unwrap().position;
        let axis = if position[0] == target[0] { 2 } else { 0 };
        position[axis] += (target[axis] - position[axis]).signum();
        *time += 100;
        session
            .apply_recovery_action_at(41, *time, Action::Move { position })
            .unwrap();
        let updates: Vec<_> = messages.try_iter().collect();
        assert!(updates.iter().any(|message| matches!(message, ServerMessage::ActorUpdate(actor) if actor.actor_id == 41 && actor.position == position)));
        assert!(!updates.iter().any(|message| matches!(
            message,
            ServerMessage::LootGranted(_) | ServerMessage::ProgressionGranted(_)
        )));
    }
}

#[test]
fn challenge_world_recovery_holds_use_server_time_and_complete_without_rewards() {
    use revenant_challenges::recovery::{Action, DELIVERY, DWELL_MS, INTAKE, SPAWN, TRANSFER};
    let Some(f) = Fixture::new() else {
        return;
    };
    let (mut session, messages) = f.enter(ContractId::CoolantRecovery, None);
    let entry: Vec<_> = messages.try_iter().collect();
    assert!(entry.iter().any(|message| matches!(message, ServerMessage::ChallengeSnapshot(board) if board.policy_revision == revenant_challenges::EXPANDED_REVISION && board.contracts.len() == 3)));
    assert_eq!(session.actors.get(41).unwrap().position, SPAWN);
    session.move_player(41, TRANSFER).unwrap();
    assert_eq!(session.actors.get(41).unwrap().position, SPAWN);
    assert!(messages.try_iter().next().is_none());
    let mut time = 100;
    recovery_walk(&mut session, &messages, INTAKE, &mut time);
    recovery_walk(&mut session, &messages, TRANSFER, &mut time);
    time += DWELL_MS - 1;
    session
        .apply_recovery_action_at(41, time, Action::Observe)
        .unwrap();
    assert_eq!(f.state().active.unwrap().objectives, 1);
    assert!(messages.try_iter().next().is_none());
    recovery_walk(&mut session, &messages, [3, 0, -5], &mut time);
    recovery_walk(&mut session, &messages, TRANSFER, &mut time);
    session
        .apply_recovery_action_at(41, time + 1, Action::Observe)
        .unwrap();
    assert_eq!(f.state().active.unwrap().objectives, 1);
    time += DWELL_MS;
    session
        .apply_recovery_action_at(41, time, Action::Observe)
        .unwrap();
    assert_eq!(f.state().active.unwrap().objectives, 3);
    let _: Vec<_> = messages.try_iter().collect();
    recovery_walk(&mut session, &messages, DELIVERY, &mut time);
    session
        .apply_recovery_action_at(41, time + DWELL_MS, Action::Observe)
        .unwrap();
    assert_eq!(session.stage, SessionStage::Complete);
    assert_eq!(f.state().last_result.unwrap().outcome, Outcome::Completed);
    assert_eq!(f.state().records.len(), 1);
    assert!(messages.try_iter().any(|message| matches!(message, ServerMessage::ChallengeSnapshot(board) if board.active.is_none() && board.records.len() == 1)));
    let mut db = Persistence::connect_existing(&f.url).unwrap();
    assert_eq!(
        db.progression_for(&f.character)
            .unwrap()
            .unwrap()
            .experience,
        0
    );
    session.disconnect(41).unwrap();
    let (mut meridian, messages) = f.enter(ContractId::MeridianCircuit, None);
    for position in [[-28, 0, -7], [-28, 0, 7], [-30, 0, 0], [-16, 0, 0]] {
        walk(&mut meridian, &messages, position);
    }
    assert_eq!(meridian.stage, SessionStage::Complete);
    let saved = f.state();
    assert_eq!(saved.records.len(), 2);
    assert!(saved
        .records
        .iter()
        .any(|run| run.rules.contract == ContractId::CoolantRecovery));
}

#[test]
fn challenge_world_recovery_retry_clears_holds_and_failed_storage_withholds_movement() {
    use revenant_challenges::recovery::{Action, INTAKE, SPAWN, TRANSFER};
    let Some(f) = Fixture::new() else {
        return;
    };
    let (mut session, messages) = f.enter(ContractId::CoolantRecovery, None);
    let _: Vec<_> = messages.try_iter().collect();
    let mut time = 0;
    recovery_walk(&mut session, &messages, INTAKE, &mut time);
    recovery_walk(&mut session, &messages, TRANSFER, &mut time);
    session
        .apply_recovery_action_at(41, time + 900, Action::Observe)
        .unwrap();
    session.disconnect(41).unwrap();
    let (mut retry, messages) = f.enter(ContractId::CoolantRecovery, Some(1));
    let _: Vec<_> = messages.try_iter().collect();
    assert_eq!(retry.actors.get(41).unwrap().position, SPAWN);
    assert_eq!(f.state().active.unwrap().objectives, 0);
    f.end_externally(&retry);
    assert!(retry
        .apply_recovery_action_at(
            41,
            100,
            Action::Move {
                position: [-1, 0, -4]
            }
        )
        .is_err());
    assert_eq!(retry.actors.get(41).unwrap().position, SPAWN);
    assert!(messages.try_iter().next().is_none());
    assert_eq!(f.state().last_result.unwrap().outcome, Outcome::Abandoned);
}

#[path = "challenge_elite.rs"]
mod elite;

#[path = "challenge_prism.rs"]
mod prism;

#[path = "challenge_signal.rs"]
mod signal;

#[path = "challenge_survival.rs"]
mod survival;

#[path = "challenge_gauntlet.rs"]
mod gauntlet;
