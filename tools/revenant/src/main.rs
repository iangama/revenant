use std::env;
use std::error::Error;
use std::io;
use std::process::ExitCode;

use revenant_persistence::Persistence;
use revenant_replay::{reconstruct, ReplayEvent, ReplayEventKind};

const DEFAULT_DATABASE_URL: &str = "postgres://revenant:revenant_local@127.0.0.1:5432/revenant";

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
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_owned());
    let mut persistence = Persistence::connect(&database_url)?;
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
                activity_id: event.activity_id,
                actor_id: event.actor_id.map(u64::try_from).transpose()?,
                payload: event.payload,
            })
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    let state = reconstruct(&events)?;
    println!("Replay session {}", state.session_id);
    for line in state.timeline {
        println!("{line}");
    }
    println!(
        "State: activity={} enemies_spawned={} enemies_defeated={} boss_spawned={} completed={}",
        state.activity_id.as_deref().unwrap_or("unknown"),
        state.spawned_enemies,
        state.defeated_enemies,
        state.boss_spawned,
        state.completed
    );
    Ok(())
}
