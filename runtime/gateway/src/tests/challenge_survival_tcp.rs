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
fn challenge_tcp_survival_confirms_holds_healing_completion_and_saved_record() {
    let Ok(url) = env::var("DATABASE_URL") else {
        return;
    };
    Persistence::connect(&url).unwrap();
    let username = format!(
        "m37-s-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let mut peer = Peer::enter(&url, &username, ContractId::LastReserve, None);
    assert_eq!(
        peer.state.as_ref().unwrap().policy_revision,
        revenant_challenges::SURVIVAL_REVISION
    );
    let ServerMessage::ChallengeSurvivalState(initial) =
        peer.until(|m| matches!(m, ServerMessage::ChallengeSurvivalState(_)))
    else {
        unreachable!()
    };
    assert_eq!(
        (initial.sequence, initial.health, initial.reserve_used),
        (1, 100, false)
    );
    let target = peer.enemies["signal-sentinel"];
    peer.send(&ClientMessage::AttackIntent(AttackIntent {
        target_actor_id: target,
    }));
    walk(&mut peer, [-8, 0, 4]);
    walk(&mut peer, [-8, 0, 0]);
    peer.until(|m| matches!(m,ServerMessage::ChallengeSurvivalState(s) if s.phase=="east"));
    walk(&mut peer, [-8, 0, 5]);
    walk(&mut peer, [-4, 0, 5]);
    let ServerMessage::ChallengeSurvivalState(healed) = peer
        .until(|m| matches!(m,ServerMessage::ChallengeSurvivalState(s) if s.recovered_health > 0))
    else {
        unreachable!()
    };
    assert!(healed.reserve_used && healed.recovered_health <= 36 && healed.health <= 100);
    walk(&mut peer, [0, 0, 5]);
    walk(&mut peer, [0, 0, 0]);
    peer.until(|m| matches!(m,ServerMessage::ChallengeSurvivalState(s) if s.phase=="return"));
    walk(&mut peer, [0, 0, 4]);
    walk(&mut peer, [-4, 0, 4]);
    peer.until(|m| matches!(m,ServerMessage::ChallengeSnapshot(s) if s.active.is_none()));
    let record = peer.state.as_ref().unwrap().records[0].clone();
    assert_eq!(record.contract.contract_id, "last_reserve");
    assert_eq!(record.contract.seed, 37007);
    drop(peer);
    let mut peer = Peer::enter(&url, &username, ContractId::LastReserve, None);
    assert_eq!(peer.state.as_ref().unwrap().records[0], record);
    let ServerMessage::ChallengeSurvivalState(fresh) =
        peer.until(|m| matches!(m, ServerMessage::ChallengeSurvivalState(_)))
    else {
        unreachable!()
    };
    assert!(fresh.run_id > initial.run_id);
    assert_eq!(
        (fresh.phase.as_str(), fresh.health, fresh.reserve_used),
        ("west", 100, false)
    );
    drop(peer);
    let mut db = Persistence::connect_existing(&url).unwrap();
    assert_eq!(
        db.progression_for(&format!("local:{username}:operator"))
            .unwrap()
            .unwrap()
            .experience,
        0
    );
}
