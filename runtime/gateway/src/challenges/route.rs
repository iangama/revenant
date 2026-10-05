//! Authoritative modified-route movement, published only after its atomic save.
use crate::SharedSession;
use revenant_challenges::ContractId;
use revenant_persistence::ModuleMutationReplayContext;
use revenant_protocol::{ActorUpdate, ServerMessage};
use revenant_replay::RouteEvidence;
use std::{error::Error, io, time::Instant};

pub(super) struct RouteRuntime {
    started: Instant,
    sequence: u64,
    pub(super) elapsed_ms: u64,
}

impl RouteRuntime {
    pub(super) fn new() -> Self {
        Self {
            started: Instant::now(),
            sequence: 0,
            elapsed_ms: 0,
        }
    }
}

impl SharedSession {
    pub(crate) fn move_challenge_route(
        &mut self,
        player_id: u64,
        position: [i32; 3],
    ) -> Result<(), Box<dyn Error>> {
        let runtime = self
            .challenge
            .as_ref()
            .ok_or_else(|| io::Error::other("route runtime missing"))?;
        let Some(run) = runtime.state.active.as_ref() else {
            return Ok(());
        };
        let route = runtime
            .route
            .as_ref()
            .ok_or_else(|| io::Error::other("route clock missing"))?;
        let player = &self.participants[0];
        let actor = self
            .actors
            .get(player_id)
            .ok_or_else(|| io::Error::other("route actor missing"))?;
        if player.actor.id != player_id
            || actor.health == 0
            || actor.position == position
            || !revenant_activities::meridian::walkable(position)
            || !revenant_activities::meridian::valid_step(actor.position, position)
        {
            return Ok(());
        }
        let proof = RouteEvidence {
            run_id: run.run_id,
            sequence: route
                .sequence
                .checked_add(1)
                .ok_or_else(|| io::Error::other("route sequence exhausted"))?,
            elapsed_ms: u64::try_from(route.started.elapsed().as_millis())
                .unwrap_or(u64::MAX)
                .max(route.elapsed_ms),
            actor_id: player_id,
            position,
            checkpoint: run.rules.visit_objective(run.objectives, position),
        };
        let receipt = self.persistence.apply_challenge_route_step(
            &player.character_id,
            &proof,
            ModuleMutationReplayContext {
                catalog_revision: player.arsenal_catalog(),
                session_id: &self.session_id,
                account_id: &player.account_id,
                activity_id: ContractId::MeridianCircuit.activity_id(),
                actor_id: i64::try_from(player_id)?,
            },
        )?;
        let runtime = self.challenge.as_mut().unwrap();
        runtime.state = receipt.state;
        let route = runtime.route.as_mut().unwrap();
        route.sequence = proof.sequence;
        route.elapsed_ms = proof.elapsed_ms;
        self.actors.update_position(player_id, position);
        self.broadcast(ServerMessage::ActorUpdate(ActorUpdate {
            actor_id: player_id,
            position,
            charge: None,
            defense: None,
        }));
        self.project_challenge();
        Ok(())
    }
}
