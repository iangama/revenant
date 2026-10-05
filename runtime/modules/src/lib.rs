use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

pub const CATALOG_REVISION: &str = "m26-v1";
pub const CONTENT_CATALOG_REVISION: &str = "m31-v1";
pub const ARSENAL_CATALOG_REVISION: &str = "m35-v1";
pub const BUILD_CATALOG_REVISION: &str = "m35-v2";
pub mod acquisition;
mod builds;
pub use builds::{all_build_loadouts, BUILD_MODULE_CATALOG};
pub const MAX_EQUIPPED_MODULES: usize = 3;
pub const CANONICAL_BUILD_COUNT: usize = 15;
pub const MAX_OPERATION_RECORDS_PER_KIND: usize = 128;
pub const MAX_LOADOUT_REVISION: u64 = i64::MAX.unsigned_abs();

const BASIS_POINTS: i32 = 10_000;
const MIN_DAMAGE: u32 = 18;
const MAX_DAMAGE: u32 = 52;
const MIN_RANGE: i32 = 4;
const MAX_RANGE: i32 = 10;
const MIN_COOLDOWN_MS: u64 = 120;
const MAX_COOLDOWN_MS: u64 = 350;
const MIN_MAX_HEALTH: u32 = 100;
const MAX_MAX_HEALTH: u32 = 130;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleFamily {
    Force,
    Tempo,
    Reach,
    Ward,
    Focus,
    Cycle,
    Breach,
    Optic,
    Skirmish,
    Ablative,
}

impl Display for ModuleFamily {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Force => "force",
            Self::Tempo => "tempo",
            Self::Reach => "reach",
            Self::Ward => "ward",
            Self::Focus => "focus",
            Self::Cycle => "cycle",
            Self::Breach => "breach",
            Self::Optic => "optic",
            Self::Skirmish => "skirmish",
            Self::Ablative => "ablative",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ModuleId {
    #[serde(rename = "module_force_matrix")]
    ForceMatrix,
    #[serde(rename = "module_tempo_regulator")]
    TempoRegulator,
    #[serde(rename = "module_reach_lattice")]
    ReachLattice,
    #[serde(rename = "module_ward_capacitor")]
    WardCapacitor,
    #[serde(rename = "module_focus_lens")]
    FocusLens,
    #[serde(rename = "module_cycle_bypass")]
    CycleBypass,
    #[serde(rename = "module_breach_shunt")]
    BreachShunt,
    #[serde(rename = "module_standoff_optic")]
    StandoffOptic,
    #[serde(rename = "module_skirmish_drive")]
    SkirmishDrive,
    #[serde(rename = "module_ablative_shell")]
    AblativeShell,
}

impl ModuleId {
    pub const LEGACY: [Self; 4] = [
        Self::ForceMatrix,
        Self::TempoRegulator,
        Self::ReachLattice,
        Self::WardCapacitor,
    ];

    pub const CONTENT: [Self; 6] = [
        Self::ForceMatrix,
        Self::TempoRegulator,
        Self::ReachLattice,
        Self::WardCapacitor,
        Self::FocusLens,
        Self::CycleBypass,
    ];

    pub const ALL: [Self; 10] = [
        Self::ForceMatrix,
        Self::TempoRegulator,
        Self::ReachLattice,
        Self::WardCapacitor,
        Self::FocusLens,
        Self::CycleBypass,
        Self::BreachShunt,
        Self::StandoffOptic,
        Self::SkirmishDrive,
        Self::AblativeShell,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ForceMatrix => "module_force_matrix",
            Self::TempoRegulator => "module_tempo_regulator",
            Self::ReachLattice => "module_reach_lattice",
            Self::WardCapacitor => "module_ward_capacitor",
            Self::FocusLens => "module_focus_lens",
            Self::CycleBypass => "module_cycle_bypass",
            Self::BreachShunt => "module_breach_shunt",
            Self::StandoffOptic => "module_standoff_optic",
            Self::SkirmishDrive => "module_skirmish_drive",
            Self::AblativeShell => "module_ablative_shell",
        }
    }

    #[must_use]
    pub const fn family(self) -> ModuleFamily {
        match self {
            Self::ForceMatrix => ModuleFamily::Force,
            Self::TempoRegulator => ModuleFamily::Tempo,
            Self::ReachLattice => ModuleFamily::Reach,
            Self::WardCapacitor => ModuleFamily::Ward,
            Self::FocusLens => ModuleFamily::Focus,
            Self::CycleBypass => ModuleFamily::Cycle,
            Self::BreachShunt => ModuleFamily::Breach,
            Self::StandoffOptic => ModuleFamily::Optic,
            Self::SkirmishDrive => ModuleFamily::Skirmish,
            Self::AblativeShell => ModuleFamily::Ablative,
        }
    }
}

impl Display for ModuleId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for ModuleId {
    type Err = ModuleError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|module_id| module_id.as_str() == value)
            .ok_or_else(|| ModuleError::UnknownModule(value.to_owned()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleDefinition {
    pub module_id: ModuleId,
    pub family: ModuleFamily,
    pub recipe_fragments: u32,
    pub damage_basis_points: i32,
    pub cooldown_basis_points: i32,
    pub range_delta: i32,
    pub max_health_delta: i32,
}

pub const MODULE_CATALOG: [ModuleDefinition; 4] = [
    ModuleDefinition {
        module_id: ModuleId::ForceMatrix,
        family: ModuleFamily::Force,
        recipe_fragments: 2,
        damage_basis_points: 2_000,
        cooldown_basis_points: 2_000,
        range_delta: 0,
        max_health_delta: 0,
    },
    ModuleDefinition {
        module_id: ModuleId::TempoRegulator,
        family: ModuleFamily::Tempo,
        recipe_fragments: 2,
        damage_basis_points: -1_000,
        cooldown_basis_points: -1_500,
        range_delta: 0,
        max_health_delta: 0,
    },
    ModuleDefinition {
        module_id: ModuleId::ReachLattice,
        family: ModuleFamily::Reach,
        recipe_fragments: 2,
        damage_basis_points: -1_000,
        cooldown_basis_points: 0,
        range_delta: 2,
        max_health_delta: 0,
    },
    ModuleDefinition {
        module_id: ModuleId::WardCapacitor,
        family: ModuleFamily::Ward,
        recipe_fragments: 2,
        damage_basis_points: 0,
        cooldown_basis_points: 1_000,
        range_delta: 0,
        max_health_delta: 20,
    },
];

pub const CONTENT_MODULE_CATALOG: [ModuleDefinition; 6] = [
    MODULE_CATALOG[0],
    MODULE_CATALOG[1],
    MODULE_CATALOG[2],
    MODULE_CATALOG[3],
    ModuleDefinition {
        module_id: ModuleId::FocusLens,
        family: ModuleFamily::Focus,
        recipe_fragments: 2,
        damage_basis_points: 2_000,
        cooldown_basis_points: 0,
        range_delta: -2,
        max_health_delta: 0,
    },
    ModuleDefinition {
        module_id: ModuleId::CycleBypass,
        family: ModuleFamily::Cycle,
        recipe_fragments: 2,
        damage_basis_points: 0,
        cooldown_basis_points: -2_000,
        range_delta: -2,
        max_health_delta: 0,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseCombatProfile {
    pub damage: u32,
    pub range: i32,
    pub cooldown_ms: u64,
    pub max_health: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AggregateModifiers {
    pub damage_basis_points: i32,
    pub cooldown_basis_points: i32,
    pub range_delta: i32,
    pub max_health_delta: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveCombatProfile {
    pub damage: u32,
    pub range: i32,
    pub cooldown_ms: u64,
    pub max_health: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedBuild {
    pub catalog_revision: &'static str,
    pub modules: Vec<ModuleId>,
    pub modifiers: AggregateModifiers,
    pub profile: EffectiveCombatProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityPhase {
    Waiting,
    Active,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationDisposition {
    Applied,
    Replayed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationReceipt<T> {
    pub disposition: OperationDisposition,
    pub outcome: T,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombinationOutcome {
    pub operation_id: String,
    pub module_id: ModuleId,
    pub recipe_fragments: u32,
    pub previous_fragments: u32,
    pub resulting_fragments: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadoutOutcome {
    pub operation_id: String,
    pub previous_modules: Vec<ModuleId>,
    pub resulting_modules: Vec<ModuleId>,
    pub previous_revision: u64,
    pub resulting_revision: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct CombinationRequest<'a> {
    pub operation_id: &'a str,
    pub module_id: ModuleId,
}

#[derive(Debug, Clone, Copy)]
pub struct LoadoutRequest<'a> {
    pub operation_id: &'a str,
    pub expected_revision: u64,
    pub modules: &'a [ModuleId],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StoredCombination {
    module_id: ModuleId,
    outcome: CombinationOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StoredLoadout {
    expected_revision: u64,
    modules: Vec<ModuleId>,
    outcome: LoadoutOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleState {
    fragments: u32,
    owned_modules: BTreeSet<ModuleId>,
    loadout: Vec<ModuleId>,
    revision: u64,
    combination_operations: BTreeMap<String, StoredCombination>,
    loadout_operations: BTreeMap<String, StoredLoadout>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleError {
    UnknownModule(String),
    UnknownCatalogRevision(String),
    TooManyModules(usize),
    DuplicateModule(ModuleId),
    InvalidCatalog(&'static str),
    ArithmeticOverflow,
    EffectiveOutOfBounds(&'static str),
}

impl Display for ModuleError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownModule(value) => write!(formatter, "unknown module: {value}"),
            Self::UnknownCatalogRevision(value) => {
                write!(formatter, "unknown module catalog revision: {value}")
            }
            Self::TooManyModules(count) => {
                write!(formatter, "module loadout exceeds three entries: {count}")
            }
            Self::DuplicateModule(module_id) => {
                write!(formatter, "module loadout repeats {module_id}")
            }
            Self::InvalidCatalog(message) => write!(formatter, "invalid module catalog: {message}"),
            Self::ArithmeticOverflow => formatter.write_str("module arithmetic overflowed"),
            Self::EffectiveOutOfBounds(field) => {
                write!(formatter, "effective module {field} is outside M26 bounds")
            }
        }
    }
}

impl Error for ModuleError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleMutationError {
    Module(ModuleError),
    InvalidOperationId,
    MutationLocked(ActivityPhase),
    AlreadyOwned(ModuleId),
    InsufficientFragments { required: u32, available: u32 },
    UnownedModule(ModuleId),
    StaleRevision { expected: u64, actual: u64 },
    UnchangedLoadout,
    IdempotencyConflict,
    OperationLimitReached,
    ArithmeticOverflow,
}

impl Display for ModuleMutationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Module(error) => error.fmt(formatter),
            Self::InvalidOperationId => formatter.write_str(
                "operation identifier must contain 1-32 ASCII alphanumeric/hyphen characters",
            ),
            Self::MutationLocked(phase) => {
                write!(formatter, "module mutation is locked during {phase:?}")
            }
            Self::AlreadyOwned(module_id) => {
                write!(formatter, "module is already owned: {module_id}")
            }
            Self::InsufficientFragments {
                required,
                available,
            } => write!(
                formatter,
                "module recipe requires {required} fragments but only {available} are available"
            ),
            Self::UnownedModule(module_id) => {
                write!(
                    formatter,
                    "module loadout contains unowned item: {module_id}"
                )
            }
            Self::StaleRevision { expected, actual } => write!(
                formatter,
                "module loadout revision is stale: expected {expected}, actual {actual}"
            ),
            Self::UnchangedLoadout => formatter.write_str("module loadout is unchanged"),
            Self::IdempotencyConflict => {
                formatter.write_str("operation identifier was already used with different input")
            }
            Self::OperationLimitReached => {
                formatter.write_str("module operation ledger reached its bounded limit")
            }
            Self::ArithmeticOverflow => {
                formatter.write_str("module mutation arithmetic overflowed")
            }
        }
    }
}

impl Error for ModuleMutationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Module(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ModuleError> for ModuleMutationError {
    fn from(error: ModuleError) -> Self {
        Self::Module(error)
    }
}

impl ModuleState {
    /// Creates validated character-owned module state without an operation history.
    ///
    /// # Errors
    ///
    /// Returns an error for duplicate ownership, invalid/duplicate/over-capacity
    /// loadout entries, or a loadout that contains an unowned module.
    pub fn new(
        fragments: u32,
        owned_modules: &[ModuleId],
        loadout: &[ModuleId],
        revision: u64,
    ) -> Result<Self, ModuleMutationError> {
        let mut owned = BTreeSet::new();
        for module_id in owned_modules {
            if !owned.insert(*module_id) {
                return Err(ModuleError::DuplicateModule(*module_id).into());
            }
        }
        let canonical = canonicalize_loadout(loadout)?;
        if let Some(unowned) = canonical
            .iter()
            .find(|module_id| !owned.contains(module_id))
        {
            return Err(ModuleMutationError::UnownedModule(*unowned));
        }
        Ok(Self {
            fragments,
            owned_modules: owned,
            loadout: canonical,
            revision,
            combination_operations: BTreeMap::new(),
            loadout_operations: BTreeMap::new(),
        })
    }

    #[must_use]
    pub const fn fragments(&self) -> u32 {
        self.fragments
    }

    #[must_use]
    pub fn owned_modules(&self) -> Vec<ModuleId> {
        self.owned_modules.iter().copied().collect()
    }

    #[must_use]
    pub fn loadout(&self) -> &[ModuleId] {
        &self.loadout
    }

    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub fn combination_operation_count(&self) -> usize {
        self.combination_operations.len()
    }

    #[must_use]
    pub fn loadout_operation_count(&self) -> usize {
        self.loadout_operations.len()
    }

    /// Atomically consumes recipe fragments and grants one unique module unlock.
    ///
    /// An accepted operation is retained in the bounded in-memory ledger. The
    /// same identifier and module returns its original outcome with `Replayed`;
    /// the same identifier with different input is rejected.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed/conflicting/over-limit operations, a phase
    /// other than `Complete`, existing ownership, or insufficient fragments.
    pub fn combine(
        &mut self,
        phase: ActivityPhase,
        request: CombinationRequest<'_>,
    ) -> Result<OperationReceipt<CombinationOutcome>, ModuleMutationError> {
        validate_operation_id(request.operation_id)?;
        if let Some(stored) = self.combination_operations.get(request.operation_id) {
            if stored.module_id != request.module_id {
                return Err(ModuleMutationError::IdempotencyConflict);
            }
            return Ok(OperationReceipt {
                disposition: OperationDisposition::Replayed,
                outcome: stored.outcome.clone(),
            });
        }
        if phase != ActivityPhase::Complete {
            return Err(ModuleMutationError::MutationLocked(phase));
        }
        if self.owned_modules.contains(&request.module_id) {
            return Err(ModuleMutationError::AlreadyOwned(request.module_id));
        }
        let definition = module_definition(request.module_id);
        if self.fragments < definition.recipe_fragments {
            return Err(ModuleMutationError::InsufficientFragments {
                required: definition.recipe_fragments,
                available: self.fragments,
            });
        }
        if self.combination_operations.len() >= MAX_OPERATION_RECORDS_PER_KIND {
            return Err(ModuleMutationError::OperationLimitReached);
        }
        let mut next = self.clone();
        let resulting_fragments = next
            .fragments
            .checked_sub(definition.recipe_fragments)
            .ok_or(ModuleMutationError::ArithmeticOverflow)?;
        let outcome = CombinationOutcome {
            operation_id: request.operation_id.to_owned(),
            module_id: request.module_id,
            recipe_fragments: definition.recipe_fragments,
            previous_fragments: next.fragments,
            resulting_fragments,
        };
        next.fragments = resulting_fragments;
        if !next.owned_modules.insert(request.module_id) {
            return Err(ModuleMutationError::AlreadyOwned(request.module_id));
        }
        next.combination_operations.insert(
            request.operation_id.to_owned(),
            StoredCombination {
                module_id: request.module_id,
                outcome: outcome.clone(),
            },
        );
        *self = next;
        Ok(OperationReceipt {
            disposition: OperationDisposition::Applied,
            outcome,
        })
    }

    /// Atomically replaces the complete canonical loadout and increments revision once.
    ///
    /// Reordered input is canonicalized before idempotency comparison, so an
    /// accepted request can be retried in another order without another revision.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed/conflicting/over-limit operations, invalid
    /// loadouts, non-complete phase, stale revision, unowned module, unchanged
    /// state, or revision overflow.
    pub fn set_loadout(
        &mut self,
        phase: ActivityPhase,
        request: LoadoutRequest<'_>,
    ) -> Result<OperationReceipt<LoadoutOutcome>, ModuleMutationError> {
        validate_operation_id(request.operation_id)?;
        let modules = canonicalize_loadout(request.modules)?;
        if let Some(stored) = self.loadout_operations.get(request.operation_id) {
            if stored.expected_revision != request.expected_revision || stored.modules != modules {
                return Err(ModuleMutationError::IdempotencyConflict);
            }
            return Ok(OperationReceipt {
                disposition: OperationDisposition::Replayed,
                outcome: stored.outcome.clone(),
            });
        }
        if phase != ActivityPhase::Complete {
            return Err(ModuleMutationError::MutationLocked(phase));
        }
        if request.expected_revision != self.revision {
            return Err(ModuleMutationError::StaleRevision {
                expected: request.expected_revision,
                actual: self.revision,
            });
        }
        if let Some(unowned) = modules
            .iter()
            .find(|module_id| !self.owned_modules.contains(module_id))
        {
            return Err(ModuleMutationError::UnownedModule(*unowned));
        }
        if modules == self.loadout {
            return Err(ModuleMutationError::UnchangedLoadout);
        }
        if self.loadout_operations.len() >= MAX_OPERATION_RECORDS_PER_KIND {
            return Err(ModuleMutationError::OperationLimitReached);
        }
        if self.revision >= MAX_LOADOUT_REVISION {
            return Err(ModuleMutationError::ArithmeticOverflow);
        }
        let resulting_revision = self.revision + 1;
        let outcome = LoadoutOutcome {
            operation_id: request.operation_id.to_owned(),
            previous_modules: self.loadout.clone(),
            resulting_modules: modules.clone(),
            previous_revision: self.revision,
            resulting_revision,
        };
        let mut next = self.clone();
        next.loadout.clone_from(&modules);
        next.revision = resulting_revision;
        next.loadout_operations.insert(
            request.operation_id.to_owned(),
            StoredLoadout {
                expected_revision: request.expected_revision,
                modules,
                outcome: outcome.clone(),
            },
        );
        *self = next;
        Ok(OperationReceipt {
            disposition: OperationDisposition::Applied,
            outcome,
        })
    }
}

/// Checks the bounded operation identifier accepted by domain and persistence.
///
/// # Errors
///
/// Returns [`ModuleMutationError::InvalidOperationId`] unless the value is 1-32
/// ASCII alphanumeric/hyphen characters.
pub fn validate_operation_id(value: &str) -> Result<(), ModuleMutationError> {
    if value.is_empty()
        || value.len() > 32
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(ModuleMutationError::InvalidOperationId);
    }
    Ok(())
}

/// Finds one definition in the fixed M26 catalog.
#[must_use]
pub const fn module_definition(module_id: ModuleId) -> ModuleDefinition {
    match module_id {
        ModuleId::ForceMatrix => MODULE_CATALOG[0],
        ModuleId::TempoRegulator => MODULE_CATALOG[1],
        ModuleId::ReachLattice => MODULE_CATALOG[2],
        ModuleId::WardCapacitor => MODULE_CATALOG[3],
        ModuleId::FocusLens => CONTENT_MODULE_CATALOG[4],
        ModuleId::CycleBypass => CONTENT_MODULE_CATALOG[5],
        ModuleId::BreachShunt => BUILD_MODULE_CATALOG[6],
        ModuleId::StandoffOptic => BUILD_MODULE_CATALOG[7],
        ModuleId::SkirmishDrive => BUILD_MODULE_CATALOG[8],
        ModuleId::AblativeShell => BUILD_MODULE_CATALOG[9],
    }
}

/// Validates catalog cardinality, identifiers, families, recipes, and tradeoff envelopes.
///
/// # Errors
///
/// Returns [`ModuleError::InvalidCatalog`] when any finite M26 catalog rule is violated.
pub fn validate_catalog(catalog: &[ModuleDefinition]) -> Result<(), ModuleError> {
    if ![
        ModuleId::LEGACY.len(),
        ModuleId::CONTENT.len(),
        ModuleId::ALL.len(),
    ]
    .contains(&catalog.len())
    {
        return Err(ModuleError::InvalidCatalog(
            "catalog must contain four legacy, six content, or ten build entries",
        ));
    }
    let mut module_ids = HashSet::with_capacity(catalog.len());
    let mut families = HashSet::with_capacity(catalog.len());
    for definition in catalog {
        if catalog.len() == ModuleId::LEGACY.len()
            && !ModuleId::LEGACY.contains(&definition.module_id)
        {
            return Err(ModuleError::InvalidCatalog(
                "legacy catalog contains a content module",
            ));
        }
        if catalog.len() == ModuleId::CONTENT.len()
            && !ModuleId::CONTENT.contains(&definition.module_id)
        {
            return Err(ModuleError::InvalidCatalog(
                "content catalog contains a build module",
            ));
        }
        if definition.module_id.family() != definition.family {
            return Err(ModuleError::InvalidCatalog(
                "module identifier and family disagree",
            ));
        }
        if !module_ids.insert(definition.module_id) || !families.insert(definition.family) {
            return Err(ModuleError::InvalidCatalog(
                "module identifiers and families must be unique",
            ));
        }
        if !(1..=8).contains(&definition.recipe_fragments) {
            return Err(ModuleError::InvalidCatalog(
                "recipe must cost between one and eight fragments",
            ));
        }
        if !definition_matches_family_envelope(*definition) {
            return Err(ModuleError::InvalidCatalog(
                "module modifiers do not match the family tradeoff envelope",
            ));
        }
    }
    Ok(())
}

/// Returns all canonical subsets of four modules with at most three entries.
#[must_use]
pub fn all_canonical_loadouts() -> Vec<Vec<ModuleId>> {
    (0_u8..16)
        .filter(|mask| mask.count_ones() <= u32::try_from(MAX_EQUIPPED_MODULES).unwrap_or(3))
        .map(|mask| {
            ModuleId::LEGACY
                .into_iter()
                .enumerate()
                .filter_map(|(index, module_id)| {
                    ((mask & (1_u8 << index)) != 0).then_some(module_id)
                })
                .collect()
        })
        .collect()
}

/// Enumerates the bounded content catalog for balance validation.
#[must_use]
pub fn all_content_loadouts() -> Vec<Vec<ModuleId>> {
    (0_u8..64)
        .filter(|mask| mask.count_ones() <= 3)
        .map(|mask| {
            ModuleId::CONTENT
                .into_iter()
                .enumerate()
                .filter_map(|(index, id)| ((mask & (1_u8 << index)) != 0).then_some(id))
                .collect()
        })
        .collect()
}

/// Resolves one canonical character-global module loadout against a base profile.
///
/// # Errors
///
/// Returns an error for an unknown catalog revision, invalid catalog, duplicate
/// or over-capacity loadout, checked arithmetic failure, or an effective value
/// outside the M26 envelope.
pub fn resolve_build(
    catalog_revision: &str,
    base: BaseCombatProfile,
    requested_modules: &[ModuleId],
) -> Result<ResolvedBuild, ModuleError> {
    if catalog_revision == BUILD_CATALOG_REVISION {
        return builds::resolve(base, requested_modules);
    }
    if requested_modules
        .iter()
        .any(|id| !ModuleId::CONTENT.contains(id))
    {
        return Err(ModuleError::InvalidCatalog(
            "loadout requires the build catalog",
        ));
    }

    if ![
        CATALOG_REVISION,
        CONTENT_CATALOG_REVISION,
        ARSENAL_CATALOG_REVISION,
    ]
    .contains(&catalog_revision)
    {
        return Err(ModuleError::UnknownCatalogRevision(
            catalog_revision.to_owned(),
        ));
    }
    validate_catalog(
        if [CONTENT_CATALOG_REVISION, ARSENAL_CATALOG_REVISION].contains(&catalog_revision) {
            &CONTENT_MODULE_CATALOG
        } else {
            &MODULE_CATALOG
        },
    )?;
    let modules = canonicalize_loadout(requested_modules)?;
    if catalog_revision == CATALOG_REVISION
        && modules.iter().any(|id| !ModuleId::LEGACY.contains(id))
    {
        return Err(ModuleError::InvalidCatalog(
            "loadout requires the content catalog",
        ));
    }
    let modifiers = aggregate_modifiers(&modules)?;
    if [CONTENT_CATALOG_REVISION, ARSENAL_CATALOG_REVISION].contains(&catalog_revision) {
        if !(-3_000..=4_000).contains(&modifiers.damage_basis_points)
            || !(-3_500..=3_000).contains(&modifiers.cooldown_basis_points)
            || !(-4..=2).contains(&modifiers.range_delta)
            || !(0..=30).contains(&modifiers.max_health_delta)
        {
            return Err(ModuleError::EffectiveOutOfBounds("content modifiers"));
        }
    } else {
        validate_aggregate(modifiers)?;
    }

    let damage = u32::try_from(scale_basis_points(
        u64::from(base.damage),
        modifiers.damage_basis_points,
    )?)
    .map_err(|_| ModuleError::ArithmeticOverflow)?;
    let cooldown_ms = scale_basis_points(base.cooldown_ms, modifiers.cooldown_basis_points)?;
    let range = base
        .range
        .checked_add(modifiers.range_delta)
        .ok_or(ModuleError::ArithmeticOverflow)?;
    let max_health = i64::from(base.max_health)
        .checked_add(i64::from(modifiers.max_health_delta))
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(ModuleError::ArithmeticOverflow)?;
    let profile = EffectiveCombatProfile {
        damage,
        range,
        cooldown_ms,
        max_health,
    };
    if [CONTENT_CATALOG_REVISION, ARSENAL_CATALOG_REVISION].contains(&catalog_revision) {
        // The lance trades cadence for reach. Preserve the original M26 bounds
        // when reconstructing older builds, including their failure cases.
        if !(MIN_DAMAGE..=80).contains(&profile.damage)
            || !(2..=12).contains(&profile.range)
            || !(90..=600).contains(&profile.cooldown_ms)
            || !(MIN_MAX_HEALTH..=MAX_MAX_HEALTH).contains(&profile.max_health)
        {
            return Err(ModuleError::EffectiveOutOfBounds("content profile"));
        }
    } else {
        validate_effective(profile)?;
    }
    Ok(ResolvedBuild {
        catalog_revision: if catalog_revision == ARSENAL_CATALOG_REVISION {
            ARSENAL_CATALOG_REVISION
        } else if catalog_revision == CONTENT_CATALOG_REVISION {
            CONTENT_CATALOG_REVISION
        } else {
            CATALOG_REVISION
        },
        modules,
        modifiers,
        profile,
    })
}

/// Returns whether `candidate` Pareto-dominates `other` for the same base weapon.
#[must_use]
pub const fn dominates(candidate: EffectiveCombatProfile, other: EffectiveCombatProfile) -> bool {
    let never_worse = candidate.damage >= other.damage
        && candidate.range >= other.range
        && candidate.cooldown_ms <= other.cooldown_ms
        && candidate.max_health >= other.max_health;
    let strictly_better = candidate.damage > other.damage
        || candidate.range > other.range
        || candidate.cooldown_ms < other.cooldown_ms
        || candidate.max_health > other.max_health;
    never_worse && strictly_better
}

/// Returns a sorted canonical loadout while enforcing capacity and uniqueness.
///
/// # Errors
///
/// Returns an error when more than three modules are requested or one module is repeated.
pub fn canonicalize_loadout(requested_modules: &[ModuleId]) -> Result<Vec<ModuleId>, ModuleError> {
    if requested_modules.len() > MAX_EQUIPPED_MODULES {
        return Err(ModuleError::TooManyModules(requested_modules.len()));
    }
    let mut modules = requested_modules.to_vec();
    modules.sort_unstable();
    if let Some(repeated) = modules.windows(2).find(|pair| pair[0] == pair[1]) {
        return Err(ModuleError::DuplicateModule(repeated[0]));
    }
    Ok(modules)
}

fn aggregate_modifiers(modules: &[ModuleId]) -> Result<AggregateModifiers, ModuleError> {
    let mut aggregate = AggregateModifiers::default();
    for module_id in modules {
        let definition = module_definition(*module_id);
        aggregate.damage_basis_points = aggregate
            .damage_basis_points
            .checked_add(definition.damage_basis_points)
            .ok_or(ModuleError::ArithmeticOverflow)?;
        aggregate.cooldown_basis_points = aggregate
            .cooldown_basis_points
            .checked_add(definition.cooldown_basis_points)
            .ok_or(ModuleError::ArithmeticOverflow)?;
        aggregate.range_delta = aggregate
            .range_delta
            .checked_add(definition.range_delta)
            .ok_or(ModuleError::ArithmeticOverflow)?;
        aggregate.max_health_delta = aggregate
            .max_health_delta
            .checked_add(definition.max_health_delta)
            .ok_or(ModuleError::ArithmeticOverflow)?;
    }
    Ok(aggregate)
}

const fn definition_matches_family_envelope(definition: ModuleDefinition) -> bool {
    match definition.family {
        ModuleFamily::Breach => {
            definition.damage_basis_points == BUILD_MODULE_CATALOG[6].damage_basis_points
                && definition.cooldown_basis_points == BUILD_MODULE_CATALOG[6].cooldown_basis_points
                && definition.range_delta == BUILD_MODULE_CATALOG[6].range_delta
                && definition.max_health_delta == BUILD_MODULE_CATALOG[6].max_health_delta
        }
        ModuleFamily::Optic => {
            definition.damage_basis_points == BUILD_MODULE_CATALOG[7].damage_basis_points
                && definition.cooldown_basis_points == BUILD_MODULE_CATALOG[7].cooldown_basis_points
                && definition.range_delta == BUILD_MODULE_CATALOG[7].range_delta
                && definition.max_health_delta == BUILD_MODULE_CATALOG[7].max_health_delta
        }
        ModuleFamily::Skirmish => {
            definition.damage_basis_points == BUILD_MODULE_CATALOG[8].damage_basis_points
                && definition.cooldown_basis_points == BUILD_MODULE_CATALOG[8].cooldown_basis_points
                && definition.range_delta == BUILD_MODULE_CATALOG[8].range_delta
                && definition.max_health_delta == BUILD_MODULE_CATALOG[8].max_health_delta
        }
        ModuleFamily::Ablative => {
            definition.damage_basis_points == BUILD_MODULE_CATALOG[9].damage_basis_points
                && definition.cooldown_basis_points == BUILD_MODULE_CATALOG[9].cooldown_basis_points
                && definition.range_delta == BUILD_MODULE_CATALOG[9].range_delta
                && definition.max_health_delta == BUILD_MODULE_CATALOG[9].max_health_delta
        }
        ModuleFamily::Focus => {
            definition.damage_basis_points == 2_000
                && definition.cooldown_basis_points == 0
                && definition.range_delta == -2
                && definition.max_health_delta == 0
        }
        ModuleFamily::Cycle => {
            definition.damage_basis_points == 0
                && definition.cooldown_basis_points == -2_000
                && definition.range_delta == -2
                && definition.max_health_delta == 0
        }
        ModuleFamily::Force => {
            definition.damage_basis_points >= 500
                && definition.damage_basis_points <= 2_000
                && definition.cooldown_basis_points >= 500
                && definition.cooldown_basis_points <= 2_000
                && definition.range_delta == 0
                && definition.max_health_delta == 0
        }
        ModuleFamily::Tempo => {
            definition.damage_basis_points >= -1_500
                && definition.damage_basis_points <= -500
                && definition.cooldown_basis_points >= -2_000
                && definition.cooldown_basis_points <= -500
                && definition.range_delta == 0
                && definition.max_health_delta == 0
        }
        ModuleFamily::Reach => {
            definition.damage_basis_points >= -1_500
                && definition.damage_basis_points <= -500
                && definition.cooldown_basis_points == 0
                && definition.range_delta >= 1
                && definition.range_delta <= 2
                && definition.max_health_delta == 0
        }
        ModuleFamily::Ward => {
            definition.damage_basis_points == 0
                && definition.cooldown_basis_points >= 500
                && definition.cooldown_basis_points <= 1_500
                && definition.range_delta == 0
                && definition.max_health_delta >= 10
                && definition.max_health_delta <= 30
        }
    }
}

const fn validate_aggregate(modifiers: AggregateModifiers) -> Result<(), ModuleError> {
    if modifiers.damage_basis_points < -3_000 || modifiers.damage_basis_points > 3_000 {
        return Err(ModuleError::EffectiveOutOfBounds("damage modifier"));
    }
    if modifiers.cooldown_basis_points < -3_000 || modifiers.cooldown_basis_points > 3_000 {
        return Err(ModuleError::EffectiveOutOfBounds("cooldown modifier"));
    }
    if modifiers.range_delta < 0 || modifiers.range_delta > 2 {
        return Err(ModuleError::EffectiveOutOfBounds("range modifier"));
    }
    if modifiers.max_health_delta < 0 || modifiers.max_health_delta > 30 {
        return Err(ModuleError::EffectiveOutOfBounds("health modifier"));
    }
    Ok(())
}

const fn validate_effective(profile: EffectiveCombatProfile) -> Result<(), ModuleError> {
    if profile.damage < MIN_DAMAGE || profile.damage > MAX_DAMAGE {
        return Err(ModuleError::EffectiveOutOfBounds("damage"));
    }
    if profile.range < MIN_RANGE || profile.range > MAX_RANGE {
        return Err(ModuleError::EffectiveOutOfBounds("range"));
    }
    if profile.cooldown_ms < MIN_COOLDOWN_MS || profile.cooldown_ms > MAX_COOLDOWN_MS {
        return Err(ModuleError::EffectiveOutOfBounds("cooldown"));
    }
    if profile.max_health < MIN_MAX_HEALTH || profile.max_health > MAX_MAX_HEALTH {
        return Err(ModuleError::EffectiveOutOfBounds("maximum health"));
    }
    Ok(())
}

fn scale_basis_points(base: u64, modifier: i32) -> Result<u64, ModuleError> {
    let multiplier = i128::from(BASIS_POINTS)
        .checked_add(i128::from(modifier))
        .ok_or(ModuleError::ArithmeticOverflow)?;
    if multiplier <= 0 {
        return Err(ModuleError::EffectiveOutOfBounds("basis-point multiplier"));
    }
    let product = i128::from(base)
        .checked_mul(multiplier)
        .ok_or(ModuleError::ArithmeticOverflow)?;
    let rounded = product
        .checked_add(i128::from(BASIS_POINTS / 2))
        .ok_or(ModuleError::ArithmeticOverflow)?
        / i128::from(BASIS_POINTS);
    u64::try_from(rounded).map_err(|_| ModuleError::ArithmeticOverflow)
}

#[cfg(test)]
mod tests {
    #[test]
    fn arsenal_weapons_accept_retained_module_loadouts_without_changing_old_catalogs() {
        for range in [6, 10] {
            let base = BaseCombatProfile {
                damage: 56,
                range,
                cooldown_ms: 400,
                max_health: 100,
            };
            for loadout in super::all_content_loadouts() {
                let build = resolve_build(super::ARSENAL_CATALOG_REVISION, base, &loadout).unwrap();
                assert_eq!(build.catalog_revision, super::ARSENAL_CATALOG_REVISION);
                assert!(build.profile.range >= 2 && build.profile.cooldown_ms >= 90);
            }
            assert!(resolve_build(CATALOG_REVISION, base, &[]).is_err());
        }
    }

    #[test]
    fn content_loadouts_cover_three_weapons_and_require_real_tradeoffs() {
        super::validate_catalog(&super::CONTENT_MODULE_CATALOG).unwrap();
        let loadouts = super::all_content_loadouts();
        assert_eq!(loadouts.len(), 42);
        for (damage, range, cooldown_ms) in [(40, 6, 250), (25, 8, 150), (48, 9, 350)] {
            let base = super::BaseCombatProfile {
                damage,
                range,
                cooldown_ms,
                max_health: 100,
            };
            for loadout in &loadouts {
                super::resolve_build(super::CONTENT_CATALOG_REVISION, base, loadout).unwrap();
            }
            let focus = super::resolve_build(
                super::CONTENT_CATALOG_REVISION,
                base,
                &[super::ModuleId::FocusLens],
            )
            .unwrap()
            .profile;
            assert!(focus.damage > damage && focus.range < range);
            let cycle = super::resolve_build(
                super::CONTENT_CATALOG_REVISION,
                base,
                &[super::ModuleId::CycleBypass],
            )
            .unwrap()
            .profile;
            assert!(cycle.cooldown_ms < cooldown_ms && cycle.range < range);
            assert!(super::resolve_build(
                super::CATALOG_REVISION,
                base,
                &[super::ModuleId::FocusLens]
            )
            .is_err());
        }
    }

    #[test]
    fn lance_supports_every_existing_loadout_without_changing_legacy_bounds() {
        let base = super::BaseCombatProfile {
            damage: 48,
            range: 9,
            cooldown_ms: 350,
            max_health: 100,
        };
        for loadout in super::all_canonical_loadouts() {
            let profile = super::resolve_build(super::CONTENT_CATALOG_REVISION, base, &loadout)
                .expect("lance combinations must stay in their catalog envelope")
                .profile;
            assert!(profile.damage <= 64 && profile.range <= 12 && profile.cooldown_ms <= 600);
        }
        assert!(super::resolve_build(
            super::CATALOG_REVISION,
            base,
            &[super::ModuleId::ForceMatrix]
        )
        .is_err());
    }

    use super::{
        all_canonical_loadouts, dominates, resolve_build, validate_catalog, ActivityPhase,
        BaseCombatProfile, CombinationRequest, EffectiveCombatProfile, LoadoutRequest, ModuleError,
        ModuleFamily, ModuleId, ModuleMutationError, ModuleState, OperationDisposition,
        CATALOG_REVISION, MAX_OPERATION_RECORDS_PER_KIND, MODULE_CATALOG,
    };

    const RIFLE: BaseCombatProfile = BaseCombatProfile {
        damage: 40,
        range: 6,
        cooldown_ms: 250,
        max_health: 100,
    };
    const SIDEARM: BaseCombatProfile = BaseCombatProfile {
        damage: 25,
        range: 8,
        cooldown_ms: 150,
        max_health: 100,
    };

    #[test]
    fn fixed_catalog_and_complete_state_space_are_valid() {
        validate_catalog(&MODULE_CATALOG).expect("fixed catalog should validate");
        let loadouts = all_canonical_loadouts();
        assert_eq!(loadouts.len(), 15);
        assert_eq!(loadouts.first(), Some(&Vec::new()));
        assert!(loadouts.iter().all(|loadout| loadout.len() <= 3));
        assert!(loadouts
            .iter()
            .all(|loadout| loadout.windows(2).all(|pair| pair[0] < pair[1])));
    }

    #[test]
    fn canonical_resolution_is_order_independent_and_rounds_half_up() {
        let first = resolve_build(
            CATALOG_REVISION,
            SIDEARM,
            &[ModuleId::TempoRegulator, ModuleId::ForceMatrix],
        )
        .expect("candidate should resolve");
        let second = resolve_build(
            CATALOG_REVISION,
            SIDEARM,
            &[ModuleId::ForceMatrix, ModuleId::TempoRegulator],
        )
        .expect("permuted candidate should resolve");
        assert_eq!(first, second);
        assert_eq!(first.profile.damage, 28);
        assert_eq!(first.profile.cooldown_ms, 158);
        assert_eq!(
            first.modules,
            vec![ModuleId::ForceMatrix, ModuleId::TempoRegulator]
        );
    }

    #[test]
    fn all_builds_stay_inside_effective_bounds() {
        for base in [RIFLE, SIDEARM] {
            for loadout in all_canonical_loadouts() {
                let resolved = resolve_build(CATALOG_REVISION, base, &loadout)
                    .expect("every canonical build should resolve");
                assert!((18..=52).contains(&resolved.profile.damage));
                assert!((4..=10).contains(&resolved.profile.range));
                assert!((120..=350).contains(&resolved.profile.cooldown_ms));
                assert!((100..=130).contains(&resolved.profile.max_health));
            }
        }
    }

    #[test]
    fn invalid_revision_capacity_duplicate_and_base_are_rejected() {
        assert_eq!(
            resolve_build("unknown", RIFLE, &[]),
            Err(ModuleError::UnknownCatalogRevision("unknown".to_owned()))
        );
        assert_eq!(
            resolve_build(CATALOG_REVISION, RIFLE, &ModuleId::LEGACY),
            Err(ModuleError::TooManyModules(4))
        );
        assert_eq!(
            resolve_build(
                CATALOG_REVISION,
                RIFLE,
                &[ModuleId::ForceMatrix, ModuleId::ForceMatrix]
            ),
            Err(ModuleError::DuplicateModule(ModuleId::ForceMatrix))
        );
        assert_eq!(
            resolve_build(
                CATALOG_REVISION,
                BaseCombatProfile {
                    damage: u32::MAX,
                    ..RIFLE
                },
                &[ModuleId::ForceMatrix]
            ),
            Err(ModuleError::ArithmeticOverflow)
        );
    }

    #[test]
    fn invalid_catalog_tradeoff_is_rejected() {
        let mut invalid = MODULE_CATALOG;
        invalid[0].family = ModuleFamily::Tempo;
        assert_eq!(
            validate_catalog(&invalid),
            Err(ModuleError::InvalidCatalog(
                "module identifier and family disagree"
            ))
        );

        let mut invalid_recipe = MODULE_CATALOG;
        invalid_recipe[0].recipe_fragments = 9;
        assert_eq!(
            validate_catalog(&invalid_recipe),
            Err(ModuleError::InvalidCatalog(
                "recipe must cost between one and eight fragments"
            ))
        );

        let mut reordered = MODULE_CATALOG;
        reordered.reverse();
        validate_catalog(&reordered).expect("catalog order must not affect validation");
    }

    #[test]
    fn combination_is_atomic_idempotent_and_phase_locked() {
        let mut state = ModuleState::new(4, &[], &[], 0).expect("empty state should validate");
        let before_locked = state.clone();
        assert_eq!(
            state.combine(
                ActivityPhase::Active,
                CombinationRequest {
                    operation_id: "combine-1",
                    module_id: ModuleId::ForceMatrix,
                }
            ),
            Err(ModuleMutationError::MutationLocked(ActivityPhase::Active))
        );
        assert_eq!(state, before_locked);

        let applied = state
            .combine(
                ActivityPhase::Complete,
                CombinationRequest {
                    operation_id: "combine-1",
                    module_id: ModuleId::ForceMatrix,
                },
            )
            .expect("complete-state combination should apply");
        assert_eq!(applied.disposition, OperationDisposition::Applied);
        assert_eq!(
            (state.fragments(), state.combination_operation_count()),
            (2, 1)
        );
        assert_eq!(state.owned_modules(), vec![ModuleId::ForceMatrix]);

        let replayed = state
            .combine(
                ActivityPhase::Active,
                CombinationRequest {
                    operation_id: "combine-1",
                    module_id: ModuleId::ForceMatrix,
                },
            )
            .expect("accepted operation should replay before lifecycle validation");
        assert_eq!(replayed.disposition, OperationDisposition::Replayed);
        assert_eq!(replayed.outcome, applied.outcome);

        let before_conflict = state.clone();
        assert_eq!(
            state.combine(
                ActivityPhase::Complete,
                CombinationRequest {
                    operation_id: "combine-1",
                    module_id: ModuleId::TempoRegulator,
                }
            ),
            Err(ModuleMutationError::IdempotencyConflict)
        );
        assert_eq!(state, before_conflict);
    }

    #[test]
    fn combination_rejections_preserve_every_domain_field() {
        let mut state = ModuleState::new(1, &[ModuleId::ForceMatrix], &[], 7)
            .expect("owned state should validate");
        for (request, expected) in [
            (
                CombinationRequest {
                    operation_id: "new-force",
                    module_id: ModuleId::ForceMatrix,
                },
                ModuleMutationError::AlreadyOwned(ModuleId::ForceMatrix),
            ),
            (
                CombinationRequest {
                    operation_id: "new-tempo",
                    module_id: ModuleId::TempoRegulator,
                },
                ModuleMutationError::InsufficientFragments {
                    required: 2,
                    available: 1,
                },
            ),
            (
                CombinationRequest {
                    operation_id: "bad_id",
                    module_id: ModuleId::TempoRegulator,
                },
                ModuleMutationError::InvalidOperationId,
            ),
        ] {
            let before = state.clone();
            assert_eq!(
                state.combine(ActivityPhase::Complete, request),
                Err(expected)
            );
            assert_eq!(state, before);
        }
    }

    #[test]
    fn loadout_is_whole_canonical_revisioned_and_idempotent() {
        let owned = ModuleId::LEGACY;
        let mut state =
            ModuleState::new(0, &owned, &[], 0).expect("fully owned state should validate");
        let applied = state
            .set_loadout(
                ActivityPhase::Complete,
                LoadoutRequest {
                    operation_id: "loadout-1",
                    expected_revision: 0,
                    modules: &[
                        ModuleId::ReachLattice,
                        ModuleId::ForceMatrix,
                        ModuleId::TempoRegulator,
                    ],
                },
            )
            .expect("valid whole loadout should apply");
        assert_eq!(applied.disposition, OperationDisposition::Applied);
        assert_eq!(state.revision(), 1);
        assert_eq!(
            state.loadout(),
            &[
                ModuleId::ForceMatrix,
                ModuleId::TempoRegulator,
                ModuleId::ReachLattice,
            ]
        );

        let replayed = state
            .set_loadout(
                ActivityPhase::Active,
                LoadoutRequest {
                    operation_id: "loadout-1",
                    expected_revision: 0,
                    modules: &[
                        ModuleId::TempoRegulator,
                        ModuleId::ReachLattice,
                        ModuleId::ForceMatrix,
                    ],
                },
            )
            .expect("canonical retry should replay before lifecycle validation");
        assert_eq!(replayed.disposition, OperationDisposition::Replayed);
        assert_eq!(replayed.outcome, applied.outcome);
        assert_eq!((state.revision(), state.loadout_operation_count()), (1, 1));
    }

    #[test]
    fn every_canonical_loadout_applies_from_fully_owned_state() {
        for (index, loadout) in all_canonical_loadouts().into_iter().enumerate() {
            let initial = if loadout.is_empty() {
                vec![ModuleId::ForceMatrix]
            } else {
                Vec::new()
            };
            let mut state = ModuleState::new(0, &ModuleId::LEGACY, &initial, 0)
                .expect("matrix state should validate");
            state
                .set_loadout(
                    ActivityPhase::Complete,
                    LoadoutRequest {
                        operation_id: &format!("matrix-{index}"),
                        expected_revision: 0,
                        modules: &loadout,
                    },
                )
                .expect("every canonical loadout should apply");
            assert_eq!(state.loadout(), loadout);
            assert_eq!(state.revision(), 1);
        }
    }

    #[test]
    fn loadout_rejections_are_atomic() {
        let mut state = ModuleState::new(0, &[ModuleId::ForceMatrix], &[], 4)
            .expect("owned state should validate");
        let duplicate = [ModuleId::ForceMatrix, ModuleId::ForceMatrix];
        let too_many = ModuleId::LEGACY;
        let unowned = [ModuleId::TempoRegulator];
        let force = [ModuleId::ForceMatrix];
        let cases = [
            (
                ActivityPhase::Active,
                LoadoutRequest {
                    operation_id: "locked",
                    expected_revision: 4,
                    modules: &force,
                },
                ModuleMutationError::MutationLocked(ActivityPhase::Active),
            ),
            (
                ActivityPhase::Complete,
                LoadoutRequest {
                    operation_id: "stale",
                    expected_revision: 3,
                    modules: &force,
                },
                ModuleMutationError::StaleRevision {
                    expected: 3,
                    actual: 4,
                },
            ),
            (
                ActivityPhase::Complete,
                LoadoutRequest {
                    operation_id: "unowned",
                    expected_revision: 4,
                    modules: &unowned,
                },
                ModuleMutationError::UnownedModule(ModuleId::TempoRegulator),
            ),
            (
                ActivityPhase::Complete,
                LoadoutRequest {
                    operation_id: "duplicate",
                    expected_revision: 4,
                    modules: &duplicate,
                },
                ModuleMutationError::Module(ModuleError::DuplicateModule(ModuleId::ForceMatrix)),
            ),
            (
                ActivityPhase::Complete,
                LoadoutRequest {
                    operation_id: "four",
                    expected_revision: 4,
                    modules: &too_many,
                },
                ModuleMutationError::Module(ModuleError::TooManyModules(4)),
            ),
        ];
        for (phase, request, expected) in cases {
            let before = state.clone();
            assert_eq!(state.set_loadout(phase, request), Err(expected));
            assert_eq!(state, before);
        }
    }

    #[test]
    fn revision_and_operation_ledger_bounds_are_explicit() {
        let mut overflow = ModuleState::new(0, &[ModuleId::ForceMatrix], &[], u64::MAX)
            .expect("overflow fixture should validate");
        let before = overflow.clone();
        assert_eq!(
            overflow.set_loadout(
                ActivityPhase::Complete,
                LoadoutRequest {
                    operation_id: "overflow",
                    expected_revision: u64::MAX,
                    modules: &[ModuleId::ForceMatrix],
                }
            ),
            Err(ModuleMutationError::ArithmeticOverflow)
        );
        assert_eq!(overflow, before);

        let mut bounded = ModuleState::new(0, &[ModuleId::ForceMatrix], &[], 0)
            .expect("ledger fixture should validate");
        for operation in 0..MAX_OPERATION_RECORDS_PER_KIND {
            let modules = if operation % 2 == 0 {
                vec![ModuleId::ForceMatrix]
            } else {
                Vec::new()
            };
            bounded
                .set_loadout(
                    ActivityPhase::Complete,
                    LoadoutRequest {
                        operation_id: &format!("bounded-{operation}"),
                        expected_revision: bounded.revision(),
                        modules: &modules,
                    },
                )
                .expect("bounded operation should apply");
        }
        let requested = if bounded.loadout().is_empty() {
            vec![ModuleId::ForceMatrix]
        } else {
            Vec::new()
        };
        let before_limit = bounded.clone();
        assert_eq!(
            bounded.set_loadout(
                ActivityPhase::Complete,
                LoadoutRequest {
                    operation_id: "bounded-limit",
                    expected_revision: bounded.revision(),
                    modules: &requested,
                }
            ),
            Err(ModuleMutationError::OperationLimitReached)
        );
        assert_eq!(bounded, before_limit);
    }

    #[test]
    fn pareto_dominance_requires_no_regression_and_one_improvement() {
        let baseline = EffectiveCombatProfile {
            damage: 40,
            range: 6,
            cooldown_ms: 250,
            max_health: 100,
        };
        let better_range = EffectiveCombatProfile {
            range: 8,
            ..baseline
        };
        let tradeoff = EffectiveCombatProfile {
            damage: 48,
            cooldown_ms: 300,
            ..baseline
        };
        assert!(dominates(better_range, baseline));
        assert!(!dominates(baseline, baseline));
        assert!(!dominates(tradeoff, baseline));
        assert!(!dominates(baseline, tradeoff));
    }
}
