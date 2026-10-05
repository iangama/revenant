use postgres::{Client, NoTls};
use revenant_inventory::RELAY_CORE_FRAGMENT;
use revenant_modules::{
    acquisition::{ArcId, ArcStatus},
    ActivityPhase, ModuleId,
};
use revenant_persistence::{
    ActivityCompletion, ActivityParticipantCompletion, ModuleJoinReplayContext,
    ModuleMutationReplayContext, NewReplayEvent, Persistence,
};
use revenant_progression::ExperienceReward;
use revenant_replay::ReplayProtocolGeneration;
use std::{
    env,
    sync::{Arc, Barrier, OnceLock},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

const ACTIVITY: &str = "relay_awakening";

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
}

impl Fixture {
    fn new(fragments: i32) -> Option<Self> {
        let Ok(url) = env::var("DATABASE_URL") else {
            eprintln!("DATABASE_URL is not set; PostgreSQL acquisition test skipped");
            return None;
        };
        let suffix = unique();
        let account = format!("local:m35-acquisition-{suffix}");
        let character = format!("{account}:operator");
        let session = format!("session-m35-acquisition-{suffix}");
        database(&url)
            .ensure_local_account(&account, "Acquisition")
            .unwrap();
        Client::connect(&url, NoTls)
            .unwrap()
            .execute(
                "INSERT INTO inventory (character_id, item_id, quantity) VALUES ($1, $2, $3) \
             ON CONFLICT (character_id, item_id) DO UPDATE SET quantity = EXCLUDED.quantity",
                &[&character, &RELAY_CORE_FRAGMENT, &fragments],
            )
            .unwrap();
        Some(Self {
            url,
            account,
            character,
            session,
        })
    }

    fn context(&self) -> ModuleMutationReplayContext<'_> {
        ModuleMutationReplayContext {
            catalog_revision: Some("m35-v2"),
            session_id: &self.session,
            account_id: &self.account,
            activity_id: ACTIVITY,
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
                activity_id: ACTIVITY,
                actor_id: 41,
                player_joined_payload: "player joined as actor 41",
                protocol_generation: ReplayProtocolGeneration::V2,
            },
        )
        .unwrap();
    }

    fn survey(&self, db: &mut Persistence) {
        for step in [
            "started",
            "meridian_arrival",
            "meridian_lens",
            "meridian_gallery",
            "meridian_log",
            "meridian_return",
        ] {
            db.append_replay_event(&NewReplayEvent {
                event_type: "field_activity",
                session_id: &self.session,
                account_id: &self.account,
                activity_id: Some(ACTIVITY),
                actor_id: Some(41),
                payload: &format!("meridian-v1:{step}"),
            })
            .unwrap();
        }
    }

    fn complete(&self, db: &mut Persistence) {
        db.complete_activity_with_rewards_and_replay(
            &NewReplayEvent {
                event_type: "activity_completed",
                session_id: &self.session,
                account_id: &self.account,
                activity_id: Some(ACTIVITY),
                actor_id: None,
                payload: "activity completed",
            },
            &[ActivityParticipantCompletion {
                completion: ActivityCompletion {
                    session_id: &self.session,
                    account_id: &self.account,
                    character_id: &self.character,
                    activity_id: ACTIVITY,
                    item_id: RELAY_CORE_FRAGMENT,
                    item_quantity: 1,
                    experience_reward: ExperienceReward::validated(1).unwrap(),
                },
                actor_id: 41,
            }],
        )
        .unwrap();
    }

    fn prepare(&self, db: &mut Persistence) {
        self.join(db);
        self.survey(db);
        self.complete(db);
    }

    fn balance(&self, db: &mut Persistence) -> u32 {
        db.module_state_for(&self.character).unwrap().fragments
    }

    fn claims(&self, db: &mut Persistence) -> usize {
        db.replay_events(&self.session)
            .unwrap()
            .iter()
            .filter(|e| e.event_type == "acquisition_claimed")
            .count()
    }
}

#[test]
fn legacy_progress_claim_crafting_retry_and_reconnect_keep_exact_balance() {
    let Some(mut f) = Fixture::new(0) else {
        return;
    };
    let mut db = database(&f.url);
    assert!(db
        .acquisition_state_for(&f.account, &f.character)
        .unwrap()
        .proofs
        .is_empty());
    f.prepare(&mut db);
    let ready = db
        .refresh_acquisition_progress(&f.account, &f.character)
        .unwrap();
    assert_eq!(
        ready.domain().unwrap().status(ArcId::Meridian),
        ArcStatus::Ready
    );
    assert_eq!(
        ready.domain().unwrap().status(ArcId::Routes),
        ArcStatus::Locked
    );
    assert_eq!(f.balance(&mut db), 1);
    let grant = db
        .claim_acquisition_with_replay(&f.character, ArcId::Meridian, f.context())
        .unwrap();
    assert_eq!(
        (
            grant.replayed,
            grant.granted_fragments,
            grant.resulting_fragments
        ),
        (false, 2, 3)
    );
    db.combine_module_with_replay(
        &f.character,
        ActivityPhase::Complete,
        "spend-commission",
        ModuleId::ForceMatrix,
        f.context(),
    )
    .unwrap();
    let spent = f.balance(&mut db);
    assert!(spent < 3);
    let retry = db
        .claim_acquisition_with_replay(&f.character, ArcId::Meridian, f.context())
        .unwrap();
    assert_eq!(
        (
            retry.replayed,
            retry.granted_fragments,
            retry.resulting_fragments
        ),
        (true, 0, spent)
    );
    assert_eq!(f.claims(&mut db), 1);
    let summary = db
        .authoritative_session_summary(&f.session)
        .unwrap()
        .unwrap();
    assert_eq!(summary.loot_grant_count, 1);
    assert_eq!(summary.acquisition_claim_count, 1);
    drop(db);
    let mut db = database(&f.url);
    assert_eq!(
        db.refresh_acquisition_progress(&f.account, &f.character)
            .unwrap()
            .claimed,
        [ArcId::Meridian]
    );
    f.session = format!("session-reconnect-{}", unique());
    f.join(&mut db);
    f.complete(&mut db);
    let retry = db
        .claim_acquisition_with_replay(&f.character, ArcId::Meridian, f.context())
        .unwrap();
    assert_eq!(
        (
            retry.replayed,
            retry.granted_fragments,
            retry.resulting_fragments
        ),
        (true, 0, spent + 1)
    );
    assert_eq!(f.claims(&mut db), 0);
}

struct FailInsert {
    url: String,
    table: &'static str,
    name: String,
}
impl FailInsert {
    fn install(f: &Fixture, table: &'static str) -> Self {
        let name = format!("acquisition_fail_{}", unique());
        let condition = if table == "replay_events" {
            format!(
                "NEW.session_id = '{}' AND NEW.event_type = 'acquisition_claimed'",
                f.session
            )
        } else {
            format!("NEW.character_id = '{}'", f.character)
        };
        Client::connect(&f.url, NoTls).unwrap().batch_execute(&format!(
            "CREATE FUNCTION {name}() RETURNS TRIGGER LANGUAGE plpgsql AS $body$ \
             BEGIN IF {condition} THEN RAISE EXCEPTION 'injected acquisition failure'; END IF; RETURN NEW; END; $body$; \
             CREATE TRIGGER {name} BEFORE INSERT ON {table} FOR EACH ROW EXECUTE FUNCTION {name}();"
        )).unwrap();
        Self {
            url: f.url.clone(),
            table,
            name,
        }
    }
}
impl Drop for FailInsert {
    fn drop(&mut self) {
        Client::connect(&self.url, NoTls)
            .unwrap()
            .batch_execute(&format!(
                "DROP TRIGGER {} ON {}; DROP FUNCTION {}();",
                self.name, self.table, self.name,
            ))
            .unwrap();
    }
}

#[test]
fn replay_and_ledger_failure_roll_back_inventory_and_allow_retry() {
    let Some(f) = Fixture::new(0) else {
        return;
    };
    let mut db = database(&f.url);
    f.prepare(&mut db);
    db.record_acquisition_progress(&f.account, &f.character, &f.session)
        .unwrap();
    for table in ["replay_events", "acquisition_claims"] {
        {
            let _failure = FailInsert::install(&f, table);
            assert!(db
                .claim_acquisition_with_replay(&f.character, ArcId::Meridian, f.context())
                .is_err());
        }
        assert_eq!(f.balance(&mut db), 1);
        assert_eq!(f.claims(&mut db), 0);
        assert!(db
            .acquisition_state_for(&f.account, &f.character)
            .unwrap()
            .claimed
            .is_empty());
    }
    assert!(
        !db.claim_acquisition_with_replay(&f.character, ArcId::Meridian, f.context())
            .unwrap()
            .replayed
    );
    assert_eq!(f.balance(&mut db), 3);
}

#[test]
fn concurrent_claims_credit_once() {
    let Some(f) = Fixture::new(0) else {
        return;
    };
    let mut db = database(&f.url);
    f.prepare(&mut db);
    db.record_acquisition_progress(&f.account, &f.character, &f.session)
        .unwrap();
    let f = Arc::new(f);
    let barrier = Arc::new(Barrier::new(2));
    let threads = (0..2)
        .map(|_| {
            let f = Arc::clone(&f);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let mut db = database(&f.url);
                barrier.wait();
                db.claim_acquisition_with_replay(&f.character, ArcId::Meridian, f.context())
                    .unwrap()
            })
        })
        .collect::<Vec<_>>();
    let receipts = threads
        .into_iter()
        .map(|t| t.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(receipts.iter().filter(|r| !r.replayed).count(), 1);
    assert_eq!(receipts.iter().map(|r| r.granted_fragments).sum::<u32>(), 2);
    assert_eq!(f.balance(&mut db), 3);
    assert_eq!(f.claims(&mut db), 1);
}

#[test]
fn incomplete_foreign_and_corrupt_proofs_never_unlock_or_grant() {
    let Some(f) = Fixture::new(0) else {
        return;
    };
    let mut db = database(&f.url);
    f.join(&mut db);
    f.survey(&mut db);
    assert!(db
        .record_acquisition_progress(&f.account, &f.character, &f.session)
        .unwrap()
        .proofs
        .is_empty());
    assert!(db
        .claim_acquisition_with_replay(&f.character, ArcId::Meridian, f.context())
        .is_err());
    f.complete(&mut db);
    let stranger = Fixture::new(0).unwrap();
    assert!(db
        .record_acquisition_progress(&stranger.account, &f.character, &f.session)
        .is_err());
    assert!(db
        .record_acquisition_progress(&stranger.account, &stranger.character, &f.session)
        .unwrap()
        .proofs
        .is_empty());
    db.record_acquisition_progress(&f.account, &f.character, &f.session)
        .unwrap();
    Client::connect(&f.url, NoTls)
        .unwrap()
        .execute(
            "UPDATE replay_events SET payload = 'meridian-v1:meridian_gallery' \
         WHERE session_id = $1 AND payload = 'meridian-v1:meridian_arrival'",
            &[&f.session],
        )
        .unwrap();
    assert!(db.acquisition_state_for(&f.account, &f.character).is_err());
    assert!(db
        .claim_acquisition_with_replay(&f.character, ArcId::Meridian, f.context())
        .is_err());
    assert_eq!(f.balance(&mut db), 1);
    assert_eq!(f.claims(&mut db), 0);
}

#[test]
fn fragment_overflow_keeps_ready_claim_unconsumed() {
    let Some(f) = Fixture::new(i32::MAX - 1) else {
        return;
    };
    let mut db = database(&f.url);
    f.prepare(&mut db);
    db.record_acquisition_progress(&f.account, &f.character, &f.session)
        .unwrap();
    assert!(db
        .claim_acquisition_with_replay(&f.character, ArcId::Meridian, f.context())
        .is_err());
    assert_eq!(f.balance(&mut db), i32::MAX as u32);
    assert_eq!(f.claims(&mut db), 0);
    assert_eq!(
        db.acquisition_state_for(&f.account, &f.character)
            .unwrap()
            .domain()
            .unwrap()
            .status(ArcId::Meridian),
        ArcStatus::Ready
    );
}
