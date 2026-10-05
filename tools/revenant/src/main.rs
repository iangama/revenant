use std::env;
use std::error::Error;
use std::io;
use std::process::ExitCode;

use revenant_persistence::{database_url_from_environment, Persistence};
use revenant_replay::{reconstruct, ReplayEvent, ReplayEventKind};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("revenant replay failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let database_url = database_url_from_environment()?;
    let mut persistence = Persistence::connect_existing(&database_url)?;
    let session_id = match arguments.as_slice() {
        [command, session_id] if command == "replay" => session_id.clone(),
        [command, option, account_id] if command == "replay" && option == "--latest" => persistence
            .latest_completed_session_id(account_id)?
            .ok_or_else(|| io::Error::other("no completed replay session was found"))?,
        _ => {
            return Err(io::Error::other(
                "usage: revenant replay <session-id> | revenant replay --latest <account-id>",
            )
            .into())
        }
    };
    let events = persistence.replay_events(&session_id)?;
    let events = events
        .into_iter()
        .map(|event| {
            Ok(ReplayEvent {
                id: event.id,
                kind: event.event_type.parse::<ReplayEventKind>()?,
                timestamp: event.timestamp,
                session_id: event.session_id,
                account_id: event.account_id,
                activity_id: event.activity_id,
                actor_id: event.actor_id.map(u64::try_from).transpose()?,
                payload: event.payload,
            })
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    let state = reconstruct(&events)?;
    println!("Replay session {}", state.session_id);
    for line in &state.timeline {
        println!("{line}");
    }
    println!(
        "State: activity={} enemies_spawned={} enemies_defeated={} boss_spawned={} loot_grants={} progression_grants={} equipment_changes={} completed={}",
        state.activity_id.as_deref().unwrap_or("unknown"),
        state.spawned_enemies,
        state.defeated_enemies,
        state.boss_spawned,
        state.loot_grants,
        state.progression_grants,
        state.equipment_changes,
        state.completed
    );
    println!(
        "Modules: legacy={} participants={} snapshots={} combinations={} loadout_changes={}",
        state.module_replay_legacy,
        state.module_participants.len(),
        state.module_snapshots,
        state.module_combinations,
        state.module_loadout_changes
    );
    for (id, status) in &state.field_objectives {
        println!("Field objective: {id} {status}");
    }
    let cooperation_terminal = state.cooperation.terminal.as_ref();
    println!(
        "Cooperation: legacy={} state={:?} phase={:?} outcome={:?} elapsed_ms={:?} participants={} contributions={} revives={} reward_participants={}",
        state.cooperation.started.is_none(),
        state.cooperation.state,
        state.cooperation.last_phase(),
        cooperation_terminal.map(|terminal| terminal.outcome),
        cooperation_terminal.map(|terminal| terminal.terminal_elapsed_ms),
        state
            .cooperation
            .started
            .as_ref()
            .map_or(0, |started| started.participants.len()),
        state.cooperation.contribution_count(),
        cooperation_terminal.map_or_else(
            || usize::from(state.cooperation.revived.is_some()),
            |terminal| usize::from(terminal.contributions.revive_count),
        ),
        cooperation_terminal.map_or(0, |terminal| terminal.grants.len()),
    );
    Ok(())
}
