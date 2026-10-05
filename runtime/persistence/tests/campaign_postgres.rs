use postgres::{Client, NoTls};
use revenant_campaign::{CampaignState, ChapterId, Command, Milestone, RunMode};
use revenant_inventory::RELAY_CORE_FRAGMENT;
use revenant_persistence::{
    CampaignReceipt, CampaignRequest, ModuleJoinReplayContext, ModuleMutationReplayContext,
    NewReplayEvent, Persistence,
};
use revenant_replay::ReplayProtocolGeneration;
use std::{
    env,
    sync::{Arc, Barrier, OnceLock},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

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

struct Fixture {
    url: String,
    account: String,
    character: String,
    session: String,
    chapter: ChapterId,
}

impl Fixture {
    fn new() -> Option<Self> {
        let Ok(url) = env::var("DATABASE_URL") else {
            eprintln!("DATABASE_URL is not set; PostgreSQL campaign test skipped");
            return None;
        };
        let id = unique();
        let account = format!("local:m36-campaign-{id}");
        let character = format!("{account}:operator");
        database(&url)
            .ensure_local_account(&account, "Campaign")
            .unwrap();
        Some(Self {
            url,
            account,
            character,
            session: format!("campaign-{id}"),
            chapter: ChapterId::ReturnSignal,
        })
    }

    fn context(&self) -> ModuleMutationReplayContext<'_> {
        ModuleMutationReplayContext {
            catalog_revision: Some("m35-v2"),
            session_id: &self.session,
            account_id: &self.account,
            activity_id: self.chapter.as_str(),
            actor_id: 41,
        }
    }

    fn join(&self, db: &mut Persistence) {
        db.append_player_joined_with_module_snapshot(
            &self.character,
            ModuleJoinReplayContext {
                catalog_revision: Some("m35-v2"),
                session_id: &self.session,
                account_id: &self.account,
                activity_id: self.chapter.as_str(),
                actor_id: 41,
                player_joined_payload: "player joined as actor 41",
                protocol_generation: ReplayProtocolGeneration::V2,
            },
        )
        .unwrap();
    }

    fn state(&self, db: &mut Persistence) -> CampaignState {
        db.campaign_state_for(&self.account, &self.character)
            .unwrap()
    }

    fn apply(
        &self,
        db: &mut Persistence,
        operation: &str,
        command: Command,
        proof: Option<i64>,
    ) -> CampaignReceipt {
        let expected_revision = self.state(db).revision;
        db.apply_campaign_command(
            &self.character,
            &CampaignRequest {
                operation_id: operation,
                expected_revision,
                command,
                proof_event_id: proof,
            },
            self.context(),
        )
        .unwrap()
    }

    fn enter(&self, db: &mut Persistence, run: &str, mode: RunMode) {
        self.apply(
            db,
            &format!("enter-{run}"),
            Command::Enter {
                run_id: run.to_owned(),
                chapter: self.chapter,
                mode,
            },
            None,
        );
    }

    fn event(&self, db: &mut Persistence, kind: &str, actor: i64, payload: &str) -> i64 {
        db.append_replay_event(&NewReplayEvent {
            event_type: kind,
            session_id: &self.session,
            account_id: &self.account,
            activity_id: Some(self.chapter.as_str()),
            actor_id: Some(actor),
            payload,
        })
        .unwrap()
    }

    fn defeat(&self, db: &mut Persistence, boss: bool) -> i64 {
        let (kind, actor, archetype) = if boss {
            ("boss", 43, "warden")
        } else {
            ("enemy", 42, "relay-drone")
        };
        self.event(
            db,
            &format!("{kind}_spawned"),
            actor,
            &format!("{kind} spawned: {archetype}"),
        );
        self.event(db, "enemy_died", actor, &format!("enemy died: {archetype}"))
    }

    fn confirm(&self, db: &mut Persistence, milestone: Milestone, proof: i64) -> CampaignReceipt {
        let run_id = self.state(db).active.unwrap().run_id;
        self.apply(
            db,
            &format!("confirm-{proof}"),
            Command::Confirm { run_id, milestone },
            Some(proof),
        )
    }

    fn balance(&self) -> (i32, i64) {
        let row = Client::connect(&self.url, NoTls).unwrap().query_one(
            "SELECT COALESCE((SELECT quantity FROM inventory WHERE character_id = $1 AND item_id = $2), 0), \
             (SELECT experience FROM progression WHERE character_id = $1)", &[&self.character, &RELAY_CORE_FRAGMENT]).unwrap();
        (row.get(0), row.get(1))
    }

    fn ready_for_boss(&self, db: &mut Persistence) -> i64 {
        self.join(db);
        self.enter(db, "first-run", RunMode::Progress);
        let drone = self.defeat(db, false);
        self.confirm(db, Milestone::RelayGuardCleared, drone);
        self.defeat(db, true)
    }
}

#[test]
#[allow(clippy::too_many_lines)] // One continuous save across chapters, sessions and practice.
fn two_chapters_reconnect_practice_and_old_retry_keep_first_clear_rewards_once() {
    let Some(mut f) = Fixture::new() else {
        return;
    };
    let mut db = database(&f.url);
    assert_eq!(f.state(&mut db), CampaignState::default());
    f.join(&mut db);
    f.enter(&mut db, "return-one", RunMode::Progress);
    assert!(db
        .apply_campaign_command(
            &f.character,
            &CampaignRequest {
                operation_id: "enter-return-one",
                expected_revision: 0,
                command: Command::Enter {
                    run_id: "conflicting-run".to_owned(),
                    chapter: f.chapter,
                    mode: RunMode::Progress
                },
                proof_event_id: None,
            },
            f.context()
        )
        .is_err());
    let drone = f.defeat(&mut db, false);
    f.confirm(&mut db, Milestone::RelayGuardCleared, drone);
    assert_eq!(f.state(&mut db).active.unwrap().checkpoint, 1);
    drop(db);
    let mut db = database(&f.url);
    f.session = format!("resumed-{}", unique());
    f.join(&mut db);
    let boss = f.defeat(&mut db, true);
    let first = f.confirm(&mut db, Milestone::ReturnSignalRecovered, boss);
    assert_eq!(first.state.cleared_chapters, 1);
    assert!(first.rewards.is_some());
    assert_eq!(f.balance(), (1, 100));
    let return_session = f.session.clone();

    f.chapter = ChapterId::MeridianReadings;
    f.session = format!("meridian-{}", unique());
    f.join(&mut db);
    f.enter(&mut db, "meridian-one", RunMode::Progress);
    for (step, milestone) in [
        ("started", None),
        ("meridian_arrival", Some(Milestone::MeridianReached)),
        ("meridian_lens", None),
        ("meridian_gallery", Some(Milestone::SurveyLinked)),
        ("meridian_log", None),
        ("meridian_return", Some(Milestone::RecoveryDelivered)),
    ] {
        let proof = f.event(
            &mut db,
            "field_activity",
            41,
            &format!("meridian-v1:{step}"),
        );
        if let Some(milestone) = milestone {
            f.confirm(&mut db, milestone, proof);
        }
    }
    assert_eq!(f.state(&mut db).cleared_chapters, 2);
    assert_eq!(f.balance(), (2, 200));

    f.chapter = ChapterId::ReturnSignal;
    f.session = format!("practice-{}", unique());
    f.join(&mut db);
    assert!(db
        .apply_campaign_command(
            &f.character,
            &CampaignRequest {
                operation_id: "reuse-run",
                expected_revision: 7,
                command: Command::Enter {
                    run_id: "return-one".to_owned(),
                    chapter: f.chapter,
                    mode: RunMode::Practice
                },
                proof_event_id: None,
            },
            f.context()
        )
        .is_err());
    f.enter(&mut db, "practice-one", RunMode::Practice);
    let drone = f.defeat(&mut db, false);
    f.confirm(&mut db, Milestone::RelayGuardCleared, drone);
    let proof = f.defeat(&mut db, true);
    assert!(f
        .confirm(&mut db, Milestone::ReturnSignalRecovered, proof)
        .rewards
        .is_none());
    assert_eq!(f.balance(), (2, 200));
    let summary = db
        .authoritative_session_summary(&f.session)
        .unwrap()
        .unwrap();
    assert!(summary.completed);
    assert_eq!(
        (summary.loot_grant_count, summary.progression_grant_count),
        (0, 0)
    );
    let saved = f.state(&mut db);
    f.session = return_session;
    let retry = db
        .apply_campaign_command(
            &f.character,
            &CampaignRequest {
                operation_id: &format!("confirm-{boss}"),
                expected_revision: first.transition.before.revision,
                command: first.transition.command,
                proof_event_id: Some(boss),
            },
            f.context(),
        )
        .unwrap();
    assert!(retry.replayed && retry.rewards.is_none());
    assert_eq!(retry.state, saved);
    assert_eq!(f.balance(), (2, 200));
}

struct FailCheckpoint {
    url: String,
    name: String,
}
impl FailCheckpoint {
    fn install(f: &Fixture) -> Self {
        Self::install_for(f, "campaign_checkpoint")
    }

    fn install_for(f: &Fixture, event_type: &str) -> Self {
        let name = format!("campaign_fail_{}", unique());
        Client::connect(&f.url, NoTls).unwrap().batch_execute(&format!(
            "CREATE FUNCTION {name}() RETURNS TRIGGER LANGUAGE plpgsql AS $body$ \
             BEGIN IF NEW.session_id = '{}' AND NEW.event_type = '{event_type}' \
             THEN RAISE EXCEPTION 'injected campaign failure'; END IF; RETURN NEW; END; $body$; \
             CREATE TRIGGER {name} BEFORE INSERT ON replay_events FOR EACH ROW EXECUTE FUNCTION {name}();", f.session)).unwrap();
        Self {
            url: f.url.clone(),
            name,
        }
    }
}
impl Drop for FailCheckpoint {
    fn drop(&mut self) {
        Client::connect(&self.url, NoTls)
            .unwrap()
            .batch_execute(&format!(
                "DROP TRIGGER {} ON replay_events; DROP FUNCTION {}();",
                self.name, self.name
            ))
            .unwrap();
    }
}

#[test]
fn terminal_write_failure_and_concurrent_retry_commit_exactly_one_clear() {
    let Some(f) = Fixture::new() else {
        return;
    };
    let mut db = database(&f.url);
    let proof = f.ready_for_boss(&mut db);
    let saved = f.state(&mut db);
    let request = || CampaignRequest {
        operation_id: "finish",
        expected_revision: 2,
        command: Command::Confirm {
            run_id: "first-run".to_owned(),
            milestone: Milestone::ReturnSignalRecovered,
        },
        proof_event_id: Some(proof),
    };
    {
        let _failure = FailCheckpoint::install(&f);
        assert!(db
            .apply_campaign_command(&f.character, &request(), f.context())
            .is_err());
    }
    assert_eq!(f.state(&mut db), saved);
    assert_eq!(f.balance(), (0, 0));
    assert!(
        !db.authoritative_session_summary(&f.session)
            .unwrap()
            .unwrap()
            .completed
    );
    let count: i64 = Client::connect(&f.url, NoTls)
        .unwrap()
        .query_one(
            "SELECT (SELECT COUNT(*) FROM inventory_reward_grants WHERE character_id = $1) \
         + (SELECT COUNT(*) FROM progression_reward_grants WHERE character_id = $1) \
         + (SELECT COUNT(*) FROM activity_history WHERE character_id = $1)",
            &[&f.character],
        )
        .unwrap()
        .get(0);
    assert_eq!(count, 0);
    let f = Arc::new(f);
    let barrier = Arc::new(Barrier::new(2));
    let threads = (0..2)
        .map(|_| {
            let f = Arc::clone(&f);
            let barrier = Arc::clone(&barrier);
            let request = request();
            thread::spawn(move || {
                let mut db = database(&f.url);
                barrier.wait();
                db.apply_campaign_command(&f.character, &request, f.context())
                    .unwrap()
            })
        })
        .collect::<Vec<_>>();
    let receipts = threads
        .into_iter()
        .map(|t| t.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(receipts.iter().filter(|r| r.replayed).count(), 1);
    assert_eq!(receipts.iter().filter(|r| r.rewards.is_some()).count(), 1);
    assert_eq!(f.balance(), (1, 100));
    assert_eq!(f.state(&mut db).cleared_chapters, 1);
}

#[test]
fn foreign_stale_and_old_world_proofs_reject_without_advancing() {
    let Some(f) = Fixture::new() else {
        return;
    };
    let mut db = database(&f.url);
    f.join(&mut db);
    let old = f.defeat(&mut db, false);
    f.enter(&mut db, "first-run", RunMode::Progress);
    let saved = f.state(&mut db);
    let stranger = Fixture::new().unwrap();
    assert!(db
        .campaign_state_for(&stranger.account, &f.character)
        .is_err());
    for (revision, proof) in [(0, old), (1, old), (1, i64::MAX)] {
        assert!(db
            .apply_campaign_command(
                &f.character,
                &CampaignRequest {
                    operation_id: "reject",
                    expected_revision: revision,
                    command: Command::Confirm {
                        run_id: "first-run".to_owned(),
                        milestone: Milestone::RelayGuardCleared
                    },
                    proof_event_id: Some(proof),
                },
                f.context()
            )
            .is_err());
        assert_eq!(f.state(&mut db), saved);
    }
    let proof = f.defeat(&mut db, false);
    Client::connect(&f.url, NoTls)
        .unwrap()
        .execute(
            "UPDATE replay_events SET account_id = $2 WHERE id = $1",
            &[&proof, &stranger.account],
        )
        .unwrap();
    assert!(db
        .apply_campaign_command(
            &f.character,
            &CampaignRequest {
                operation_id: "reject",
                expected_revision: 1,
                command: Command::Confirm {
                    run_id: "first-run".to_owned(),
                    milestone: Milestone::RelayGuardCleared
                },
                proof_event_id: Some(proof),
            },
            f.context()
        )
        .is_err());
    assert_eq!(f.state(&mut db), saved);
    assert_eq!(f.balance(), (0, 0));
}

#[test]
fn corrupted_reward_ledger_prevents_loading_or_awarding_later_chapters() {
    let Some(f) = Fixture::new() else {
        return;
    };
    let mut db = database(&f.url);
    let proof = f.ready_for_boss(&mut db);
    f.confirm(&mut db, Milestone::ReturnSignalRecovered, proof);
    assert_eq!(f.state(&mut db).cleared_chapters, 1);
    Client::connect(&f.url, NoTls)
        .unwrap()
        .execute(
            "DELETE FROM progression_reward_grants WHERE character_id = $1",
            &[&f.character],
        )
        .unwrap();
    assert!(db.campaign_state_for(&f.account, &f.character).is_err());
    assert!(db
        .apply_campaign_command(
            &f.character,
            &CampaignRequest {
                operation_id: "next",
                expected_revision: 3,
                command: Command::Enter {
                    run_id: "next".to_owned(),
                    chapter: ChapterId::MeridianReadings,
                    mode: RunMode::Progress
                },
                proof_event_id: None,
            },
            f.context()
        )
        .is_err());
    assert_eq!(f.balance(), (1, 100));
}

#[test]
fn inventory_overflow_preserves_checkpoint_and_unconsumed_first_clear() {
    let Some(f) = Fixture::new() else {
        return;
    };
    Client::connect(&f.url, NoTls)
        .unwrap()
        .execute(
            "INSERT INTO inventory (character_id, item_id, quantity) VALUES ($1, $2, $3) \
         ON CONFLICT (character_id, item_id) DO UPDATE SET quantity = EXCLUDED.quantity",
            &[&f.character, &RELAY_CORE_FRAGMENT, &i32::MAX],
        )
        .unwrap();
    let mut db = database(&f.url);
    let proof = f.ready_for_boss(&mut db);
    let saved = f.state(&mut db);
    assert!(db
        .apply_campaign_command(
            &f.character,
            &CampaignRequest {
                operation_id: "overflow",
                expected_revision: saved.revision,
                command: Command::Confirm {
                    run_id: "first-run".to_owned(),
                    milestone: Milestone::ReturnSignalRecovered
                },
                proof_event_id: Some(proof),
            },
            f.context()
        )
        .is_err());
    assert_eq!(f.state(&mut db), saved);
    assert_eq!(f.balance(), (i32::MAX, 0));
    assert!(
        !db.authoritative_session_summary(&f.session)
            .unwrap()
            .unwrap()
            .completed
    );
}

#[test]
#[allow(clippy::too_many_lines)] // Three sessions exercise the actual durable resume chain.
fn meridian_resumes_after_arrival_and_survey_with_verified_admission_and_one_reward() {
    let Some(mut f) = Fixture::new() else {
        return;
    };
    let mut db = database(&f.url);
    let proof = f.ready_for_boss(&mut db);
    f.confirm(&mut db, Milestone::ReturnSignalRecovered, proof);
    f.chapter = ChapterId::MeridianReadings;
    f.session = format!("meridian-entry-{}", unique());
    f.join(&mut db);
    f.enter(&mut db, "meridian-run", RunMode::Progress);
    f.event(&mut db, "field_activity", 41, "meridian-v1:started");
    let proof = f.event(
        &mut db,
        "field_activity",
        41,
        "meridian-v1:meridian_arrival",
    );
    f.confirm(&mut db, Milestone::MeridianReached, proof);
    let arrival = f.state(&mut db);
    assert_eq!(arrival.active.as_ref().unwrap().checkpoint, 1);
    // An admission cannot be inserted after gameplay has already begun.
    assert!(db
        .resume_campaign_with_replay(&f.character, arrival.revision, f.context())
        .is_err());

    f.session = format!("meridian-arrival-resume-{}", unique());
    f.join(&mut db);
    assert!(db
        .resume_campaign_with_replay(&f.character, arrival.revision + 1, f.context())
        .is_err());
    {
        let _failure = FailCheckpoint::install_for(&f, "campaign_resumed");
        assert!(db
            .resume_campaign_with_replay(&f.character, arrival.revision, f.context())
            .is_err());
    }
    assert_eq!(f.state(&mut db), arrival);
    assert!(!db
        .replay_events(&f.session)
        .unwrap()
        .iter()
        .any(|e| e.event_type == "campaign_resumed"));
    for _ in 0..2 {
        assert_eq!(
            db.resume_campaign_with_replay(&f.character, arrival.revision, f.context())
                .unwrap(),
            arrival
        );
    }
    assert_eq!(
        db.replay_events(&f.session)
            .unwrap()
            .iter()
            .filter(|e| e.event_type == "campaign_resumed")
            .count(),
        1
    );
    f.event(&mut db, "field_activity", 41, "meridian-v1:meridian_lens");
    let proof = f.event(
        &mut db,
        "field_activity",
        41,
        "meridian-v1:meridian_gallery",
    );
    f.confirm(&mut db, Milestone::SurveyLinked, proof);
    let survey = f.state(&mut db);
    assert_eq!(survey.active.as_ref().unwrap().checkpoint, 2);
    assert_eq!(f.balance(), (1, 100));
    // Retried admission remains its original boundary, without moving the save back.
    assert_eq!(
        db.resume_campaign_with_replay(&f.character, arrival.revision, f.context())
            .unwrap(),
        arrival
    );
    assert_eq!(f.state(&mut db), survey);
    drop(db);

    let mut db = database(&f.url);
    f.session = format!("meridian-survey-resume-{}", unique());
    f.join(&mut db);
    assert_eq!(
        db.resume_campaign_with_replay(&f.character, survey.revision, f.context())
            .unwrap(),
        survey
    );
    f.event(&mut db, "field_activity", 41, "meridian-v1:meridian_log");
    let proof = f.event(&mut db, "field_activity", 41, "meridian-v1:meridian_return");
    let terminal = f.confirm(&mut db, Milestone::RecoveryDelivered, proof);
    assert_eq!(terminal.state.cleared_chapters, 2);
    assert_eq!(f.balance(), (2, 200));
    let events = db.replay_events(&f.session).unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|e| e.event_type == "field_activity")
            .map(|e| e.payload.as_str())
            .collect::<Vec<_>>(),
        ["meridian-v1:meridian_log", "meridian-v1:meridian_return"]
    );
    let admission = events
        .iter()
        .find(|e| e.event_type == "campaign_resumed")
        .unwrap();
    let mut payload = revenant_replay::decode_campaign_resume(&admission.payload).unwrap();
    payload.checkpoint_event_id -= 1; // Valid JSON/state, but the wrong durable proof.
    let corrupt = revenant_replay::encode_campaign_resume(&payload).unwrap();
    Client::connect(&f.url, NoTls)
        .unwrap()
        .execute(
            "UPDATE replay_events SET payload = $2 WHERE id = $1",
            &[&admission.id, &corrupt],
        )
        .unwrap();
    assert!(db.campaign_state_for(&f.account, &f.character).is_err());
    assert_eq!(f.balance(), (2, 200));
}

#[test]
#[allow(clippy::too_many_lines)] // One continuous legacy opening, four resumes and practice.
fn supply_boundaries_resume_and_terminal_retry_preserve_one_grant() {
    let Some(mut f) = Fixture::new() else {
        return;
    };
    let mut db = database(&f.url);
    let boss = f.ready_for_boss(&mut db);
    f.confirm(&mut db, Milestone::ReturnSignalRecovered, boss);
    f.chapter = ChapterId::MeridianReadings;
    f.session = format!("meridian-{}", unique());
    f.join(&mut db);
    f.enter(&mut db, "meridian-opening", RunMode::Progress);
    for (step, milestone) in [
        ("started", None),
        ("meridian_arrival", Some(Milestone::MeridianReached)),
        ("meridian_lens", None),
        ("meridian_gallery", Some(Milestone::SurveyLinked)),
        ("meridian_log", None),
        ("meridian_return", Some(Milestone::RecoveryDelivered)),
    ] {
        let proof = f.event(
            &mut db,
            "field_activity",
            41,
            &format!("meridian-v1:{step}"),
        );
        if let Some(milestone) = milestone {
            f.confirm(&mut db, milestone, proof);
        }
    }
    assert_eq!(f.balance(), (2, 200));
    f.chapter = ChapterId::BrokenSupplyLine;
    for mode in [RunMode::Progress, RunMode::Practice] {
        f.session = format!("supply-{}", unique());
        f.join(&mut db);
        f.enter(&mut db, &format!("supply-{}", unique()), mode);
        for (checkpoint, milestone) in f.chapter.milestones().iter().enumerate() {
            let saved = f.state(&mut db);
            assert_eq!(
                usize::from(saved.active.as_ref().unwrap().checkpoint),
                checkpoint
            );
            f.session = format!("supply-resume-{}", unique());
            f.join(&mut db);
            assert_eq!(
                db.resume_campaign_with_replay(&f.character, saved.revision, f.context())
                    .unwrap(),
                saved
            );
            let proof = match milestone {
                Milestone::SupplyGuardCleared => {
                    f.event(
                        &mut db,
                        "enemy_spawned",
                        42,
                        "enemy spawned: signal-sentinel",
                    );
                    f.event(&mut db, "enemy_died", 42, "enemy died: signal-sentinel")
                }
                _ => f.event(
                    &mut db,
                    "campaign_objective",
                    41,
                    match milestone {
                        Milestone::SupplyRouteReached => "supply-v1:route:-4,0,4",
                        Milestone::SupplyCellRecovered => "supply-v1:cell:-4,0,-3",
                        Milestone::SupplyDelivered => "supply-v1:delivery:-1,0,-5",
                        _ => unreachable!(),
                    },
                ),
            };
            // A proof for the current visit cannot justify the later delivery.
            if checkpoint == 0 {
                let invalid = CampaignRequest {
                    operation_id: "skip-supply",
                    expected_revision: saved.revision,
                    command: Command::Confirm {
                        run_id: saved.active.as_ref().unwrap().run_id.clone(),
                        milestone: Milestone::SupplyDelivered,
                    },
                    proof_event_id: Some(proof),
                };
                assert!(db
                    .apply_campaign_command(&f.character, &invalid, f.context())
                    .is_err());
                assert_eq!(f.state(&mut db), saved);
            }
            let receipt = f.confirm(&mut db, *milestone, proof);
            let expected_reward = mode == RunMode::Progress && checkpoint == 3;
            assert_eq!(receipt.rewards.is_some(), expected_reward);
            let retry = CampaignRequest {
                operation_id: &format!("confirm-{proof}"),
                expected_revision: saved.revision,
                command: receipt.transition.command.clone(),
                proof_event_id: Some(proof),
            };
            let retried = db
                .apply_campaign_command(&f.character, &retry, f.context())
                .unwrap();
            assert!(retried.replayed && retried.rewards.is_none());
        }
        assert_eq!(f.balance(), (3, 300));
        assert_eq!(f.state(&mut db).cleared_chapters, 3);
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn story_transaction_rolls_back_proof_retries_once_and_resumes_without_rewards() {
    use revenant_campaign::story;
    let Some(mut f) = Fixture::new() else {
        return;
    };
    let mut db = database(&f.url);
    let boss = f.ready_for_boss(&mut db);
    f.confirm(&mut db, Milestone::ReturnSignalRecovered, boss);
    f.chapter = ChapterId::MeridianReadings;
    f.session = format!("story-{}", unique());
    f.join(&mut db);
    f.enter(&mut db, "story-meridian", RunMode::Progress);
    f.event(&mut db, "field_activity", 41, "meridian-v1:started");
    let arrival = f.event(
        &mut db,
        "field_activity",
        41,
        "meridian-v1:meridian_arrival",
    );
    f.confirm(&mut db, Milestone::MeridianReached, arrival);
    f.event(&mut db, "field_activity", 41, "meridian-v1:meridian_lens");
    let survey = f.event(
        &mut db,
        "field_activity",
        41,
        "meridian-v1:meridian_gallery",
    );
    f.confirm(&mut db, Milestone::SurveyLinked, survey);
    let before = f.state(&mut db);
    let command = Command::Story {
        run_id: "story-meridian".to_owned(),
        position: story::DEPARTURE_BOARD,
        action: story::Command::ChooseSupply {
            approach: story::SupplyApproach::Service,
        },
    };
    let request = CampaignRequest {
        operation_id: "story-choice",
        expected_revision: before.revision,
        command: command.clone(),
        proof_event_id: None,
    };
    let fault = FailCheckpoint::install(&f);
    assert!(db
        .apply_campaign_command(&f.character, &request, f.context())
        .is_err());
    assert_eq!(f.state(&mut db), before);
    let count = || -> i64 {
        Client::connect(&f.url, NoTls).unwrap().query_one(
        "SELECT count(*) FROM replay_events WHERE session_id = $1 AND event_type = 'campaign_story'", &[&f.session]).unwrap().get(0)
    };
    assert_eq!(count(), 0);
    drop(fault);
    let first = db
        .apply_campaign_command(&f.character, &request, f.context())
        .unwrap();
    let retry = db
        .apply_campaign_command(&f.character, &request, f.context())
        .unwrap();
    assert!(!first.replayed && retry.replayed);
    assert_eq!(first.state, retry.state);
    assert!(first.rewards.is_none() && retry.rewards.is_none());
    assert_eq!(count(), 1);
    assert_eq!(f.balance(), (1, 100));
    let stale = CampaignRequest {
        operation_id: "story-stale",
        ..request
    };
    assert!(db
        .apply_campaign_command(&f.character, &stale, f.context())
        .is_err());
    assert_eq!(count(), 1);
    let mut conflict = stale;
    conflict.operation_id = "story-choice";
    conflict.command = Command::Story {
        run_id: "story-meridian".to_owned(),
        position: story::DEPARTURE_BOARD,
        action: story::Command::ChooseSupply {
            approach: story::SupplyApproach::Covered,
        },
    };
    assert!(db
        .apply_campaign_command(&f.character, &conflict, f.context())
        .is_err());
    let saved = f.state(&mut db);
    f.session = format!("story-resume-{}", unique());
    f.join(&mut db);
    assert_eq!(
        db.resume_campaign_with_replay(&f.character, saved.revision, f.context())
            .unwrap(),
        saved
    );
    for (operation, position, discovery) in [
        (
            "story-find",
            story::MEMORY,
            story::Discovery::EmptySeatMemory,
        ),
        (
            "story-return",
            story::DEPARTURE_BOARD,
            story::Discovery::EmptySeatReturned,
        ),
        (
            "story-revisit",
            story::MEMORY,
            story::Discovery::EmptySeatMemory,
        ),
    ] {
        let receipt = f.apply(
            &mut db,
            operation,
            Command::Story {
                run_id: "story-meridian".to_owned(),
                position,
                action: story::Command::Discover { discovery },
            },
            None,
        );
        assert!(receipt.rewards.is_none());
    }
    assert_eq!(
        f.state(&mut db).story.empty_seat,
        story::ArcProgress::Returned
    );
    f.event(&mut db, "field_activity", 41, "meridian-v1:meridian_log");
    let delivery = f.event(&mut db, "field_activity", 41, "meridian-v1:meridian_return");
    f.confirm(&mut db, Milestone::RecoveryDelivered, delivery);
    assert_eq!(f.balance(), (2, 200));
    assert_eq!(
        f.state(&mut db).story.supply,
        Some(story::SupplyApproach::Service)
    );
}
