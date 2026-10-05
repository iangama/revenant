use postgres::{Client, NoTls};
use revenant_challenges::{
    ChallengeState, Command, ContractId, EndReason, Equipment, Objective, Outcome,
};
use revenant_modules::BUILD_CATALOG_REVISION;
use revenant_persistence::{
    ChallengeRequest, ModuleJoinReplayContext, ModuleMutationReplayContext, NewReplayEvent,
    Persistence,
};
use revenant_replay::{
    encode_support_evidence, ReplayProtocolGeneration, SupportEvidence, SupportTransition,
};
use std::{
    env,
    sync::{Arc, Barrier, OnceLock},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

#[path = "challenges_postgres/elite.rs"]
mod elite;

#[path = "challenges_postgres/prism.rs"]
mod prism;

#[path = "challenges_postgres/signal.rs"]
mod signal;

#[path = "challenges_postgres/gauntlet.rs"]
mod gauntlet;
#[path = "challenges_postgres/route.rs"]
mod route;
#[path = "challenges_postgres/survival.rs"]
mod survival;

fn unique() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

fn database(url: &str) -> Persistence {
    static READY: OnceLock<()> = OnceLock::new();
    READY.get_or_init(|| {
        Persistence::connect(url).unwrap();
    });
    Persistence::connect_existing(url).unwrap()
}

#[derive(Clone)]
struct Fixture {
    url: String,
    account: String,
    character: String,
    session: String,
    contract: ContractId,
}

impl Fixture {
    fn new(contract: ContractId) -> Option<Self> {
        let Ok(url) = env::var("DATABASE_URL") else {
            eprintln!("DATABASE_URL is not set; PostgreSQL challenge test skipped");
            return None;
        };
        let id = unique();
        let account = format!("local:m37-{id}");
        database(&url)
            .ensure_local_account(&account, "Challenge")
            .unwrap();
        Some(Self {
            url,
            character: format!("{account}:operator"),
            account,
            session: format!("challenge-{id}"),
            contract,
        })
    }

    fn context(&self) -> ModuleMutationReplayContext<'_> {
        ModuleMutationReplayContext {
            catalog_revision: Some(BUILD_CATALOG_REVISION),
            session_id: &self.session,
            account_id: &self.account,
            activity_id: self.contract.activity_id(),
            actor_id: 41,
        }
    }

    fn join(&self, db: &mut Persistence) {
        db.append_player_joined_with_module_snapshot(
            &self.character,
            ModuleJoinReplayContext {
                catalog_revision: Some(BUILD_CATALOG_REVISION),
                session_id: &self.session,
                account_id: &self.account,
                activity_id: self.contract.activity_id(),
                actor_id: 41,
                player_joined_payload: "player joined as actor 41",
                protocol_generation: ReplayProtocolGeneration::V2,
            },
        )
        .unwrap();
    }

    fn state(&self, db: &mut Persistence) -> ChallengeState {
        db.challenge_state_for(&self.account, &self.character)
            .unwrap()
    }

    fn equipment(&self, db: &mut Persistence) -> Equipment {
        let state = db.module_state_for(&self.character).unwrap();
        Equipment {
            catalog_revision: BUILD_CATALOG_REVISION.into(),
            loadout_revision: state.revision,
            weapon_id: db.equipped_weapon_for(&self.character).unwrap().unwrap(),
            modules: state.loadout,
        }
    }

    fn start(&self, db: &mut Persistence, op: &str) -> ChallengeState {
        let request = ChallengeRequest {
            operation_id: op,
            expected_revision: self.state(db).revision,
            command: Command::Start {
                contract: self.contract,
                equipment: self.equipment(db),
            },
            proof_event_id: None,
            position: None,
        };
        db.apply_challenge_command(&self.character, &request, self.context())
            .unwrap()
            .state
    }

    fn visit(&self, db: &mut Persistence, objective: Objective) -> ChallengeState {
        let state = self.state(db);
        let request = ChallengeRequest {
            operation_id: &format!("visit-{}", state.revision),
            expected_revision: state.revision,
            command: Command::Confirm {
                run_id: state.active.unwrap().run_id,
                objective,
            },
            proof_event_id: None,
            position: objective.position(),
        };
        db.apply_challenge_command(&self.character, &request, self.context())
            .unwrap()
            .state
    }

    fn event(&self, db: &mut Persistence, time: u64, transition: SupportTransition) -> i64 {
        let (kind, actor) = match transition {
            SupportTransition::Started { lancer_id, .. } => {
                ("enemy_spawned", i64::try_from(lancer_id).unwrap())
            }
            SupportTransition::Attack {
                target_id,
                health_after: 0,
                ..
            } => ("enemy_died", i64::try_from(target_id).unwrap()),
            _ => ("field_activity", 41),
        };
        db.append_replay_event(&NewReplayEvent {
            event_type: kind,
            session_id: &self.session,
            account_id: &self.account,
            activity_id: Some(self.contract.activity_id()),
            actor_id: Some(actor),
            payload: &encode_support_evidence(&SupportEvidence {
                elapsed_ms: time,
                transition,
            })
            .unwrap(),
        })
        .unwrap()
    }

    fn counts(&self) -> (i64, i64) {
        let row = Client::connect(&self.url, NoTls).unwrap().query_one(
            "SELECT COUNT(*) FILTER (WHERE event_type = 'challenge_checkpoint'), COUNT(*) FILTER (WHERE event_type = 'challenge_objective') FROM replay_events WHERE session_id = $1", &[&self.session]).unwrap();
        (row.get(0), row.get(1))
    }
}

#[test]
fn exploration_reconnect_old_retry_and_repeat_completion_preserve_all_rewards_and_campaign() {
    let Some(mut f) = Fixture::new(ContractId::MeridianCircuit) else {
        return;
    };
    let mut db = database(&f.url);
    let inventory = db.inventory_for(&f.character).unwrap();
    let progression = db.progression_for(&f.character).unwrap();
    let campaign = db.campaign_state_for(&f.account, &f.character).unwrap();
    assert_eq!(f.state(&mut db), ChallengeState::default());
    f.join(&mut db);
    let equipment = f.equipment(&mut db);
    f.start(&mut db, "start");
    for objective in [
        Objective::LogRead,
        Objective::LensRead,
        Objective::GalleryRead,
        Objective::Returned,
    ] {
        f.visit(&mut db, objective);
    }
    let first = f.state(&mut db);
    assert_eq!(first.records.len(), 1);
    assert_eq!(
        first.last_result.as_ref().unwrap().outcome,
        Outcome::Completed
    );
    drop(db);
    let mut db = database(&f.url);
    let retry = db
        .apply_challenge_command(
            &f.character,
            &ChallengeRequest {
                operation_id: "start",
                expected_revision: 0,
                command: Command::Start {
                    contract: f.contract,
                    equipment,
                },
                proof_event_id: None,
                position: None,
            },
            f.context(),
        )
        .unwrap();
    assert!(retry.replayed);
    assert_eq!(retry.state, first);
    assert_eq!(f.counts(), (5, 4));
    f.session = format!("repeat-{}", unique());
    f.join(&mut db);
    f.start(&mut db, "repeat");
    for objective in [
        Objective::GalleryRead,
        Objective::LensRead,
        Objective::LogRead,
        Objective::Returned,
    ] {
        f.visit(&mut db, objective);
    }
    assert_eq!(f.state(&mut db).records, first.records);
    assert_eq!(db.inventory_for(&f.character).unwrap(), inventory);
    assert_eq!(db.progression_for(&f.character).unwrap(), progression);
    assert_eq!(
        db.campaign_state_for(&f.account, &f.character).unwrap(),
        campaign
    );
}

#[test]
fn combat_requires_each_fatal_hit_and_reconstructs_the_terminal_without_grants() {
    let Some(f) = Fixture::new(ContractId::CloseQuarters) else {
        return;
    };
    let mut db = database(&f.url);
    f.join(&mut db);
    f.start(&mut db, "combat");
    f.event(
        &mut db,
        0,
        SupportTransition::Started {
            lancer_id: 42,
            mender_id: 43,
            player_health: 100,
        },
    );
    let mut time = 0;
    for (target_id, objective, initial) in [
        (43, Objective::MenderCleared, 120u32),
        (42, Objective::LancerCleared, 200),
    ] {
        let mut health = initial;
        while health > 0 {
            let after = health.saturating_sub(40);
            let proof = f.event(
                &mut db,
                time,
                SupportTransition::Attack {
                    target_id,
                    player_position: [5, 0, 8],
                    damage: 40,
                    health_before: health,
                    health_after: after,
                },
            );
            let request = ChallengeRequest {
                operation_id: &format!("hit-{time}"),
                expected_revision: f.state(&mut db).revision,
                command: Command::Confirm {
                    run_id: 1,
                    objective,
                },
                proof_event_id: Some(proof),
                position: None,
            };
            let result = db.apply_challenge_command(&f.character, &request, f.context());
            if after == 0 {
                assert!(result.is_ok());
            } else {
                assert!(result.is_err());
            }
            health = after;
            time += 250;
        }
    }
    assert_eq!(f.state(&mut db).records.len(), 1);
    assert_eq!(f.counts(), (3, 0));
    let row = Client::connect(&f.url, NoTls).unwrap().query_one(
        "SELECT (SELECT COUNT(*) FROM inventory_reward_grants WHERE session_id = $1), (SELECT COUNT(*) FROM progression_reward_grants WHERE session_id = $1)", &[&f.session]).unwrap();
    assert_eq!((row.get::<_, i64>(0), row.get::<_, i64>(1)), (0, 0));
}

#[test]
#[allow(clippy::too_many_lines)] // One encounter proves rollback, transport retry and atomic terminal ordering.
fn atomic_fatal_hit_rolls_back_with_checkpoint_and_retry_never_duplicates_the_death() {
    let Some(f) = Fixture::new(ContractId::CloseQuarters) else {
        return;
    };
    let mut db = database(&f.url);
    f.join(&mut db);
    f.start(&mut db, "combat");
    f.event(
        &mut db,
        0,
        SupportTransition::Started {
            lancer_id: 42,
            mender_id: 43,
            player_health: 100,
        },
    );
    for (time, health) in [(0, 120u32), (250, 80)] {
        f.event(
            &mut db,
            time,
            SupportTransition::Attack {
                target_id: 43,
                player_position: [5, 0, 8],
                damage: 40,
                health_before: health,
                health_after: health - 40,
            },
        );
    }
    let mut evidence = SupportEvidence {
        elapsed_ms: 500,
        transition: SupportTransition::Attack {
            target_id: 43,
            player_position: [5, 0, 8],
            damage: 40,
            health_before: 40,
            health_after: 0,
        },
    };
    let request = ChallengeRequest {
        operation_id: "fatal-mender",
        expected_revision: 1,
        command: Command::Confirm {
            run_id: 1,
            objective: Objective::MenderCleared,
        },
        proof_event_id: None,
        position: None,
    };
    let deaths = || -> i64 {
        Client::connect(&f.url, NoTls).unwrap().query_one(
        "SELECT COUNT(*) FROM replay_events WHERE session_id = $1 AND event_type = 'enemy_died'", &[&f.session]).unwrap().get(0)
    };
    let mut foreign = f.context();
    foreign.actor_id = 99;
    assert!(db
        .apply_challenge_combat_command(&f.character, &request, foreign, &evidence)
        .is_err());
    assert_eq!(deaths(), 0);
    assert_eq!(f.counts(), (1, 0));
    assert_eq!(f.state(&mut db).revision, 1);
    assert!(
        !db.apply_challenge_combat_command(&f.character, &request, f.context(), &evidence)
            .unwrap()
            .replayed
    );
    assert!(
        db.apply_challenge_combat_command(&f.character, &request, f.context(), &evidence)
            .unwrap()
            .replayed
    );
    assert_eq!(deaths(), 1);
    evidence.elapsed_ms += 1;
    assert!(db
        .apply_challenge_combat_command(&f.character, &request, f.context(), &evidence)
        .is_err());
    assert_eq!(deaths(), 1);
    for (index, health) in [200u32, 160, 120, 80].into_iter().enumerate() {
        f.event(
            &mut db,
            750 + u64::try_from(index).unwrap() * 250,
            SupportTransition::Attack {
                target_id: 42,
                player_position: [5, 0, 8],
                damage: 40,
                health_before: health,
                health_after: health - 40,
            },
        );
    }
    let evidence = SupportEvidence {
        elapsed_ms: 1750,
        transition: SupportTransition::Attack {
            target_id: 42,
            player_position: [5, 0, 8],
            damage: 40,
            health_before: 40,
            health_after: 0,
        },
    };
    let request = ChallengeRequest {
        operation_id: "fatal-lancer",
        expected_revision: 2,
        command: Command::Confirm {
            run_id: 1,
            objective: Objective::LancerCleared,
        },
        proof_event_id: None,
        position: None,
    };
    let terminal = db
        .apply_challenge_combat_command(&f.character, &request, f.context(), &evidence)
        .unwrap();
    assert_eq!(terminal.state.records.len(), 1);
    assert!(terminal.state.active.is_none());
    assert!(
        db.apply_challenge_combat_command(&f.character, &request, f.context(), &evidence)
            .unwrap()
            .replayed
    );
    assert_eq!(deaths(), 2);
    assert_eq!(f.counts(), (3, 0));
}

#[test]
fn rejected_visits_roll_back_proof_and_checkpoint_and_retry_rejects_changed_inputs() {
    let Some(f) = Fixture::new(ContractId::MeridianCircuit) else {
        return;
    };
    let mut db = database(&f.url);
    f.join(&mut db);
    let state = f.start(&mut db, "start");
    let mut request = ChallengeRequest {
        operation_id: "visit",
        expected_revision: 1,
        command: Command::Confirm {
            run_id: 1,
            objective: Objective::LensRead,
        },
        proof_event_id: None,
        position: Some([0, 0, 0]),
    };
    assert!(db
        .apply_challenge_command(&f.character, &request, f.context())
        .is_err());
    request.position = Objective::LensRead.position();
    // A mismatched actor fails after the spatial proof was inserted; neither
    // inserted row may survive that failed transaction.
    let mut context = f.context();
    context.actor_id = 99;
    assert!(db
        .apply_challenge_command(&f.character, &request, context)
        .is_err());
    assert_eq!(f.counts(), (1, 0));
    assert_eq!(f.state(&mut db), state);
    let accepted = db
        .apply_challenge_command(&f.character, &request, f.context())
        .unwrap();
    assert!(
        db.apply_challenge_command(&f.character, &request, f.context())
            .unwrap()
            .replayed
    );
    request.command = Command::Confirm {
        run_id: 1,
        objective: Objective::GalleryRead,
    };
    request.position = Objective::GalleryRead.position();
    assert!(db
        .apply_challenge_command(&f.character, &request, f.context())
        .is_err());
    assert_eq!(f.state(&mut db), accepted.state);
    assert_eq!(f.counts(), (2, 1));
}

#[test]
fn retry_interrupts_old_attempt_and_old_session_cannot_advance_new_attempt() {
    let Some(mut f) = Fixture::new(ContractId::MeridianCircuit) else {
        return;
    };
    let mut db = database(&f.url);
    f.join(&mut db);
    f.start(&mut db, "first");
    f.visit(&mut db, Objective::LensRead);
    let old = f.clone();
    f.session = format!("retry-{}", unique());
    f.join(&mut db);
    let request = ChallengeRequest {
        operation_id: "retry",
        expected_revision: 2,
        command: Command::Retry {
            run_id: 1,
            equipment: f.equipment(&mut db),
        },
        proof_event_id: None,
        position: None,
    };
    let fresh = db
        .apply_challenge_command(&f.character, &request, f.context())
        .unwrap()
        .state;
    assert_eq!(fresh.active.as_ref().unwrap().run_id, 3);
    assert_eq!(fresh.active.as_ref().unwrap().objectives, 0);
    assert_eq!(
        fresh.last_result.as_ref().unwrap().outcome,
        Outcome::Interrupted
    );
    let count_before_read = total_events(&f);
    let (saved, archive) = db.challenge_archive_for(&f.account, &f.character).unwrap();
    assert_eq!(saved, fresh);
    assert!(archive.records.is_empty() && archive.badges().is_empty());
    let attempt = archive.last_attempt.unwrap();
    assert_eq!(attempt.run_id, 1);
    assert_eq!(
        attempt.assessments[0].reason,
        revenant_challenges::mastery::Reason::CompleteContract
    );
    assert_eq!(total_events(&f), count_before_read);
    let stale = ChallengeRequest {
        operation_id: "stale-visit",
        expected_revision: 3,
        command: Command::Confirm {
            run_id: 3,
            objective: Objective::GalleryRead,
        },
        proof_event_id: None,
        position: Objective::GalleryRead.position(),
    };
    assert!(db
        .apply_challenge_command(&f.character, &stale, old.context())
        .is_err());
    assert_eq!(old.counts(), (2, 1));
    f.visit(&mut db, Objective::GalleryRead);
}

#[test]
fn concurrent_terminal_operation_commits_once_and_conflicting_start_is_rejected() {
    let Some(f) = Fixture::new(ContractId::MeridianCircuit) else {
        return;
    };
    let mut db = database(&f.url);
    f.join(&mut db);
    f.start(&mut db, "start");
    for objective in [
        Objective::LensRead,
        Objective::GalleryRead,
        Objective::LogRead,
    ] {
        f.visit(&mut db, objective);
    }
    let gate = Arc::new(Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let f = f.clone();
            let gate = Arc::clone(&gate);
            thread::spawn(move || {
                let mut db = database(&f.url);
                gate.wait();
                db.apply_challenge_command(
                    &f.character,
                    &ChallengeRequest {
                        operation_id: "finish",
                        expected_revision: 4,
                        command: Command::Confirm {
                            run_id: 1,
                            objective: Objective::Returned,
                        },
                        proof_event_id: None,
                        position: Objective::Returned.position(),
                    },
                    f.context(),
                )
                .unwrap()
                .replayed
            })
        })
        .collect();
    assert_eq!(
        handles
            .into_iter()
            .map(|h| usize::from(h.join().unwrap()))
            .sum::<usize>(),
        1
    );
    assert_eq!(f.counts(), (5, 4));
    assert_eq!(f.state(&mut db).records.len(), 1);
    let request = ChallengeRequest {
        operation_id: "second-in-session",
        expected_revision: 5,
        command: Command::Start {
            contract: f.contract,
            equipment: f.equipment(&mut db),
        },
        proof_event_id: None,
        position: None,
    };
    assert!(db
        .apply_challenge_command(&f.character, &request, f.context())
        .is_err());
    assert_eq!(f.counts(), (5, 4));
}

#[test]
fn foreign_character_unaccepted_weapon_and_corrupt_journal_are_rejected() {
    let Some(f) = Fixture::new(ContractId::MeridianCircuit) else {
        return;
    };
    let mut db = database(&f.url);
    f.join(&mut db);
    let mut equipment = f.equipment(&mut db);
    equipment.weapon_id = "rail_driver".into();
    let request = ChallengeRequest {
        operation_id: "unaccepted",
        expected_revision: 0,
        command: Command::Start {
            contract: f.contract,
            equipment,
        },
        proof_event_id: None,
        position: None,
    };
    assert!(db
        .apply_challenge_command(&f.character, &request, f.context())
        .is_err());
    assert!(db
        .challenge_state_for("foreign-account", &f.character)
        .is_err());
    assert_eq!(f.counts(), (0, 0));
    f.start(&mut db, "start");
    f.visit(&mut db, Objective::LensRead);
    let mut sql = Client::connect(&f.url, NoTls).unwrap();
    sql.execute("UPDATE replay_events SET payload = jsonb_set(payload::jsonb, '{position}', '[0,0,0]'::jsonb)::text WHERE session_id = $1 AND event_type = 'challenge_objective'", &[&f.session]).unwrap();
    assert!(db.challenge_state_for(&f.account, &f.character).is_err());
    let request = ChallengeRequest {
        operation_id: "abandon-corrupt",
        expected_revision: 2,
        command: Command::End {
            run_id: 1,
            reason: EndReason::Abandoned,
        },
        proof_event_id: None,
        position: None,
    };
    assert!(db
        .apply_challenge_command(&f.character, &request, f.context())
        .is_err());
    assert_eq!(f.counts(), (2, 1));
}

struct RecoveryDriver {
    run: revenant_challenges::recovery::RecoveryRun,
    run_id: u64,
    sequence: u64,
    time: u64,
}

impl RecoveryDriver {
    fn new(run_id: u64) -> Self {
        Self {
            run: revenant_challenges::recovery::RecoveryRun::default(),
            run_id,
            sequence: 0,
            time: 0,
        }
    }

    fn proposal(
        &self,
        elapsed_ms: u64,
        action: revenant_challenges::recovery::Action,
    ) -> revenant_replay::RecoveryEvidence {
        revenant_replay::RecoveryEvidence {
            run_id: self.run_id,
            sequence: self.sequence + 1,
            elapsed_ms,
            action,
            checkpoint: self.run.apply(elapsed_ms, action).unwrap().checkpoint,
        }
    }

    fn store(
        &mut self,
        f: &Fixture,
        db: &mut Persistence,
        elapsed_ms: u64,
        action: revenant_challenges::recovery::Action,
    ) -> revenant_replay::RecoveryEvidence {
        let proof = self.proposal(elapsed_ms, action);
        assert!(
            !db.apply_challenge_recovery_step(&f.character, &proof, f.context())
                .unwrap()
                .replayed
        );
        self.run = self.run.apply(elapsed_ms, action).unwrap().run;
        self.sequence += 1;
        self.time = elapsed_ms;
        proof
    }

    fn walk(&mut self, f: &Fixture, db: &mut Persistence, target: [i32; 3]) {
        while self.run.position() != target {
            let mut position = self.run.position();
            let axis = if position[0] == target[0] { 2 } else { 0 };
            position[axis] += (target[axis] - position[axis]).signum();
            self.store(
                f,
                db,
                self.time + 100,
                revenant_challenges::recovery::Action::Move { position },
            );
        }
    }
}

fn total_events(f: &Fixture) -> i64 {
    Client::connect(&f.url, NoTls)
        .unwrap()
        .query_one(
            "SELECT COUNT(*) FROM replay_events WHERE session_id = $1",
            &[&f.session],
        )
        .unwrap()
        .get(0)
}

#[test]
#[allow(clippy::too_many_lines)] // Keep the continuous stream, failed hold and terminal race together.
fn recovery_atomic_steps_survive_reconnect_reject_forged_holds_and_converge_without_rewards() {
    use revenant_challenges::recovery::{Action, Checkpoint, DELIVERY, DWELL_MS, INTAKE, TRANSFER};
    let Some(mut f) = Fixture::new(ContractId::CoolantRecovery) else {
        return;
    };
    let mut db = database(&f.url);
    let inventory = db.inventory_for(&f.character).unwrap();
    let progression = db.progression_for(&f.character).unwrap();
    let campaign = db.campaign_state_for(&f.account, &f.character).unwrap();
    f.join(&mut db);
    f.start(&mut db, "recovery-start");
    let mut driver = RecoveryDriver::new(1);
    let first = driver.store(
        &f,
        &mut db,
        100,
        Action::Move {
            position: [-1, 0, -4],
        },
    );
    let count = total_events(&f);
    assert!(
        db.apply_challenge_recovery_step(&f.character, &first, f.context())
            .unwrap()
            .replayed
    );
    let mut changed = first;
    changed.elapsed_ms += 1;
    assert!(db
        .apply_challenge_recovery_step(&f.character, &changed, f.context())
        .is_err());
    assert_eq!(total_events(&f), count);
    driver.walk(&f, &mut db, INTAKE);
    drop(db);
    let mut db = database(&f.url);
    assert_eq!(f.state(&mut db).active.unwrap().objectives, 1);
    driver.walk(&f, &mut db, TRANSFER);
    let count = total_events(&f);
    let mut early = driver.proposal(driver.time + DWELL_MS - 1, Action::Observe);
    early.checkpoint = Some(Checkpoint::Transferred);
    assert!(db
        .apply_challenge_recovery_step(&f.character, &early, f.context())
        .is_err());
    assert_eq!(total_events(&f), count); // Both the inserted step and checkpoint rolled back.
    assert_eq!(f.state(&mut db).active.unwrap().objectives, 1);
    let direct = ChallengeRequest {
        operation_id: "instant-hold",
        expected_revision: 2,
        command: Command::Confirm {
            run_id: 1,
            objective: Objective::CoolantTransferred,
        },
        proof_event_id: None,
        position: Some(TRANSFER),
    };
    assert!(db
        .apply_challenge_command(&f.character, &direct, f.context())
        .is_err());
    driver.store(&f, &mut db, driver.time + DWELL_MS, Action::Observe);
    driver.walk(&f, &mut db, DELIVERY);
    let terminal = driver.proposal(driver.time + DWELL_MS, Action::Observe);
    let barrier = Arc::new(Barrier::new(2));
    let workers: Vec<_> = (0..2)
        .map(|_| {
            let owned = f.clone();
            let proof = terminal.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let mut db = database(&owned.url);
                barrier.wait();
                db.apply_challenge_recovery_step(&owned.character, &proof, owned.context())
                    .unwrap()
            })
        })
        .collect();
    let receipts: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(
        receipts.iter().filter(|receipt| receipt.replayed).count(),
        1
    );
    assert_eq!(receipts[0].transition, receipts[1].transition);
    let completed = f.state(&mut db);
    assert_eq!(
        completed.last_result.as_ref().unwrap().outcome,
        Outcome::Completed
    );
    assert_eq!(completed.records.len(), 1);
    assert_eq!(db.inventory_for(&f.character).unwrap(), inventory);
    assert_eq!(db.progression_for(&f.character).unwrap(), progression);
    assert_eq!(
        db.campaign_state_for(&f.account, &f.character).unwrap(),
        campaign
    );
    // An initial contract still works after an expanded-policy record exists.
    f.session = format!("after-recovery-{}", unique());
    f.contract = ContractId::MeridianCircuit;
    f.join(&mut db);
    f.start(&mut db, "meridian-after-recovery");
    for objective in [
        Objective::LensRead,
        Objective::GalleryRead,
        Objective::LogRead,
        Objective::Returned,
    ] {
        f.visit(&mut db, objective);
    }
    let final_state = f.state(&mut db);
    assert_eq!(final_state.records.len(), 2);
    assert_eq!(final_state.records[0], completed.records[0]);
}

#[test]
fn recovery_checkpoint_collision_with_an_older_session_never_breaks_the_character_journal() {
    use revenant_challenges::recovery::{Action, INTAKE};
    let Some(mut f) = Fixture::new(ContractId::MeridianCircuit) else {
        return;
    };
    let mut db = database(&f.url);
    f.join(&mut db);
    f.start(&mut db, "recovery-3-2"); // Deliberately collides with a future derived operation.
    let end = ChallengeRequest {
        operation_id: "end-old",
        expected_revision: 1,
        command: Command::End {
            run_id: 1,
            reason: EndReason::Abandoned,
        },
        proof_event_id: None,
        position: None,
    };
    db.apply_challenge_command(&f.character, &end, f.context())
        .unwrap();
    f.session = format!("collision-{}", unique());
    f.contract = ContractId::CoolantRecovery;
    f.join(&mut db);
    f.start(&mut db, "new-recovery");
    let mut driver = RecoveryDriver::new(3);
    driver.store(
        &f,
        &mut db,
        100,
        Action::Move {
            position: [-1, 0, -4],
        },
    );
    let count = total_events(&f);
    let proof = driver.proposal(200, Action::Move { position: INTAKE });
    assert!(db
        .apply_challenge_recovery_step(&f.character, &proof, f.context())
        .is_err());
    assert_eq!(total_events(&f), count);
    assert_eq!(f.state(&mut db).active.unwrap().objectives, 0);
}
