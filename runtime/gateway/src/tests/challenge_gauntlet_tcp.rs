use super::*;

fn walk(peer: &mut Peer, target: [i32; 3]) {
    while peer.position != target {
        let mut next = peer.position;
        let axis = if next[0] == target[0] { 2 } else { 0 };
        next[axis] += (target[axis] - next[axis]).signum();
        peer.step(next);
    }
}

#[test]
fn challenge_tcp_gauntlet_confirms_transfer_healing_and_retries_the_whole_run() {
    let Ok(url) = env::var("DATABASE_URL") else {
        return;
    };
    Persistence::connect(&url).unwrap();
    let username = username();
    let mut peer = Peer::enter(&url, &username, ContractId::RelayGauntlet, None);
    assert_eq!(
        peer.state.as_ref().unwrap().policy_revision,
        revenant_challenges::GAUNTLET_REVISION
    );
    let ServerMessage::ChallengeGauntletState(initial) =
        peer.until(|m| matches!(m, ServerMessage::ChallengeGauntletState(_)))
    else {
        unreachable!()
    };
    assert_eq!(
        (initial.stage, initial.health, initial.phase.as_str()),
        (1, 100, "combat")
    );
    walk(&mut peer, [5, 0, 8]);
    let player = peer.player;
    peer.until(|m| matches!(m, ServerMessage::DamageApplied(d) if d.target_actor_id == player));
    defeat_support(&mut peer);
    let ServerMessage::ChallengeGauntletState(cleared) = peer
        .until(|m| matches!(m, ServerMessage::ChallengeGauntletState(s) if s.phase == "transfer"))
    else {
        unreachable!()
    };
    assert!(cleared.health < 100);
    walk(&mut peer, revenant_ai::support::ENTRANCE);
    let ServerMessage::ChallengeGauntletState(healed) =
        peer.until(|m| matches!(m, ServerMessage::ChallengeGauntletState(s) if s.stage == 2))
    else {
        unreachable!()
    };
    assert_eq!(healed.health, 100.min(cleared.health + 24));
    assert_eq!(healed.recovered_health, healed.health - cleared.health);
    assert_eq!(
        peer.position,
        revenant_ai::elite::EliteComposition::BastionLink.entrance()
    );
    assert_eq!(
        peer.state
            .as_ref()
            .unwrap()
            .active
            .as_ref()
            .unwrap()
            .objectives,
        1
    );
    let revision = peer.state.as_ref().unwrap().state_revision;
    peer.send(&ClientMessage::ChallengeAbandonIntent(
        ChallengeAbandonIntent {
            operation_id: "gauntlet-transfer-abandon".into(),
            expected_revision: revision,
            run_id: initial.run_id,
        },
    ));
    peer.until(|m| matches!(m, ServerMessage::ChallengeActionResult(r) if r.status == "accepted"));
    drop(peer);
    let mut retry = Peer::enter(
        &url,
        &username,
        ContractId::RelayGauntlet,
        Some(initial.run_id),
    );
    let ServerMessage::ChallengeGauntletState(fresh) =
        retry.until(|m| matches!(m, ServerMessage::ChallengeGauntletState(_)))
    else {
        unreachable!()
    };
    assert!(fresh.run_id > initial.run_id);
    assert_eq!(
        (fresh.stage, fresh.health, fresh.recovered_health),
        (1, 100, 0)
    );
    assert_eq!(
        retry
            .state
            .as_ref()
            .unwrap()
            .active
            .as_ref()
            .unwrap()
            .objectives,
        0
    );
    assert!(retry.state.as_ref().unwrap().records.is_empty());
    drop(retry);
    let mut db = Persistence::connect_existing(&url).unwrap();
    assert_eq!(
        db.progression_for(&format!("local:{username}:operator"))
            .unwrap()
            .unwrap()
            .experience,
        0
    );
}

fn defeat_support(peer: &mut Peer) {
    for archetype in ["relay-mender", "glass-lancer"] {
        let target = peer.enemies[archetype];
        loop {
            peer.send(&ClientMessage::AttackIntent(AttackIntent {
                target_actor_id: target,
            }));
            let hit = peer.until(
                |m| matches!(m, ServerMessage::DamageApplied(d) if d.target_actor_id == target),
            );
            thread::sleep(Duration::from_millis(270));
            if matches!(hit, ServerMessage::DamageApplied(d) if d.killed) {
                break;
            }
        }
    }
}

fn username() -> String {
    format!(
        "m37-g-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}
