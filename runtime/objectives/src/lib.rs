#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectiveKind {
    KillActors,
    ReachArea,
    Boss,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectiveState {
    Pending,
    Active,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Objective {
    pub id: String,
    pub kind: ObjectiveKind,
    pub state: ObjectiveState,
    pub progress: u32,
    pub target: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldTrigger {
    ActorGroupDead { group_id: String },
    AreaReached { area_id: String },
}
