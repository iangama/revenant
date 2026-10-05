//! Untimed recovery circuit using the retained coolant stations. Server time
//! and accepted adjacent moves are inputs; clients never report a held station.
use serde::{Deserialize, Serialize};

pub const REVISION: &str = "coolant-recovery-v1";
pub const SPAWN: [i32; 3] = [-1, 0, -3];
pub const INTAKE: [i32; 3] = [-1, 0, -5];
pub const TRANSFER: [i32; 3] = [4, 0, -5];
pub const DELIVERY: [i32; 3] = [4, 0, -1];
pub const DWELL_MS: u64 = 1_200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Checkpoint {
    Retrieved,
    Transferred,
    Delivered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Move { position: [i32; 3] },
    Observe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Retrieve,
    Transfer,
    Delivery,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryRun {
    position: [i32; 3],
    phase: Phase,
    last_elapsed_ms: u64,
    dwell_started_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    pub run: RecoveryRun,
    pub checkpoint: Option<Checkpoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    ClockReversed,
    InvalidMovement,
    Completed,
}

impl Default for RecoveryRun {
    fn default() -> Self {
        Self {
            position: SPAWN,
            phase: Phase::Retrieve,
            last_elapsed_ms: 0,
            dwell_started_ms: None,
        }
    }
}

impl RecoveryRun {
    #[must_use]
    pub const fn position(&self) -> [i32; 3] {
        self.position
    }

    #[must_use]
    pub const fn phase(&self) -> Phase {
        self.phase
    }

    /// Plans a change without mutating the accepted state. Gateways persist the
    /// evidence before replacing their state or projecting a checkpoint.
    ///
    /// # Errors
    /// Rejects clock reversal, out-of-bounds/nonadjacent moves and actions after
    /// completion. A newly constructed run always resets position and all holds.
    pub fn apply(&self, elapsed_ms: u64, action: Action) -> Result<Update, Error> {
        if self.phase == Phase::Completed {
            return Err(Error::Completed);
        }
        if elapsed_ms < self.last_elapsed_ms {
            return Err(Error::ClockReversed);
        }
        let mut next = self.clone();
        if let Action::Move { position } = action {
            if !walkable(position)
                || (i64::from(position[0]) - i64::from(self.position[0])).abs()
                    + (i64::from(position[2]) - i64::from(self.position[2])).abs()
                    != 1
            {
                return Err(Error::InvalidMovement);
            }
            next.position = position;
        }
        next.last_elapsed_ms = elapsed_ms;
        let station = match next.phase {
            Phase::Retrieve => INTAKE,
            Phase::Transfer => TRANSFER,
            Phase::Delivery => DELIVERY,
            Phase::Completed => unreachable!(),
        };
        let checkpoint = if next.position != station {
            next.dwell_started_ms = None;
            None
        } else if next.phase == Phase::Retrieve {
            next.phase = Phase::Transfer;
            Some(Checkpoint::Retrieved)
        } else {
            let started = *next.dwell_started_ms.get_or_insert(elapsed_ms);
            if elapsed_ms - started >= DWELL_MS {
                next.dwell_started_ms = None;
                let checkpoint = if next.phase == Phase::Transfer {
                    next.phase = Phase::Delivery;
                    Checkpoint::Transferred
                } else {
                    next.phase = Phase::Completed;
                    Checkpoint::Delivered
                };
                Some(checkpoint)
            } else {
                None
            }
        };
        Ok(Update {
            run: next,
            checkpoint,
        })
    }
}

#[must_use]
pub fn walkable(position: [i32; 3]) -> bool {
    position[1] == 0 && (-2..=5).contains(&position[0]) && (-6..=0).contains(&position[2])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn walk(run: &mut RecoveryRun, target: [i32; 3], time: &mut u64) -> Vec<Checkpoint> {
        let mut confirmed = vec![];
        while run.position() != target {
            let mut position = run.position();
            let axis = if position[0] == target[0] { 2 } else { 0 };
            position[axis] += (target[axis] - position[axis]).signum();
            *time += 100;
            let next = run.apply(*time, Action::Move { position }).unwrap();
            confirmed.extend(next.checkpoint);
            *run = next.run;
        }
        confirmed
    }

    #[test]
    fn recovery_requires_order_and_uninterrupted_holds_without_a_deadline() {
        let mut run = RecoveryRun::default();
        let mut now = 100_000; // Deliberately exceeds the old timed coolant run.
        assert!(walk(&mut run, DELIVERY, &mut now).is_empty());
        assert_eq!(
            run.apply(now + DWELL_MS, Action::Observe)
                .unwrap()
                .checkpoint,
            None
        );
        now += DWELL_MS;
        assert_eq!(walk(&mut run, INTAKE, &mut now), [Checkpoint::Retrieved]);
        assert!(walk(&mut run, TRANSFER, &mut now).is_empty());
        assert_eq!(
            run.apply(now + DWELL_MS - 1, Action::Observe)
                .unwrap()
                .checkpoint,
            None
        );
        now += DWELL_MS - 1;
        walk(&mut run, [3, 0, -5], &mut now);
        walk(&mut run, TRANSFER, &mut now);
        assert_eq!(
            run.apply(now + 1, Action::Observe).unwrap().checkpoint,
            None
        );
        now += DWELL_MS;
        let next = run.apply(now, Action::Observe).unwrap();
        assert_eq!(next.checkpoint, Some(Checkpoint::Transferred));
        run = next.run;
        walk(&mut run, DELIVERY, &mut now);
        let next = run.apply(now + DWELL_MS, Action::Observe).unwrap();
        assert_eq!(next.checkpoint, Some(Checkpoint::Delivered));
        assert_eq!(next.run.phase(), Phase::Completed);
        assert_eq!(
            next.run.apply(now + DWELL_MS, Action::Observe),
            Err(Error::Completed)
        );
    }

    #[test]
    fn recovery_rejects_clock_reversal_and_teleports_without_changing_state() {
        let run = RecoveryRun::default()
            .apply(50, Action::Observe)
            .unwrap()
            .run;
        for position in [TRANSFER, [-1, 1, -3], [i32::MAX, 0, i32::MIN], SPAWN] {
            assert_eq!(
                run.apply(100, Action::Move { position }),
                Err(Error::InvalidMovement)
            );
        }
        assert_eq!(run.apply(49, Action::Observe), Err(Error::ClockReversed));
        assert_eq!(run.position(), SPAWN);
        assert_eq!(run.phase(), Phase::Retrieve);
    }

    #[test]
    fn recovery_retry_restores_spawn_and_cannot_inherit_a_partial_hold() {
        let mut run = RecoveryRun::default();
        let mut now = 0;
        walk(&mut run, INTAKE, &mut now);
        walk(&mut run, TRANSFER, &mut now);
        run = run.apply(now + 900, Action::Observe).unwrap().run;
        assert_eq!(run.phase(), Phase::Transfer);
        let retry = RecoveryRun::default();
        assert_eq!(retry.position(), SPAWN);
        assert_eq!(
            retry
                .apply(now + 10_000, Action::Observe)
                .unwrap()
                .checkpoint,
            None
        );
        assert_eq!(retry.phase(), Phase::Retrieve);
    }
}
