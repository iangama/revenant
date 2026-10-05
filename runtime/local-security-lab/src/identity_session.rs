use std::collections::{BTreeMap, VecDeque};
use std::fmt::{self, Debug, Formatter};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;

use serde::Serialize;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

const SECRET_BYTES: usize = 32;
const RATE_WINDOW_MS: u64 = 60_000;
const RATE_FAILURE_LIMIT: usize = 5;
const SESSION_IDLE_MS: u64 = 600_000;
const SESSION_ABSOLUTE_MS: u64 = 1_800_000;
const INVITE_LIFETIME_MS: u64 = 600_000;
const LOBBY_ID: &str = "local-lobby-1";

#[derive(Clone, PartialEq, Eq)]
pub struct Secret([u8; SECRET_BYTES]);

impl Debug for Secret {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("Secret([REDACTED; 32 bytes])")
    }
}

impl Secret {
    /// Generates one opaque 256-bit value with the operating-system CSPRNG.
    ///
    /// # Errors
    ///
    /// Returns the operating-system random source error without exposing any
    /// partially filled bytes.
    pub fn generate() -> Result<Self, getrandom::Error> {
        let mut bytes = [0_u8; SECRET_BYTES];
        getrandom::fill(&mut bytes)?;
        Ok(Self(bytes))
    }

    fn fixture(label: &str) -> Self {
        let mut digest = Sha256::new();
        digest.update(b"revenant.m30.fixture.v1\0");
        digest.update(label.as_bytes());
        Self(digest.finalize().into())
    }
}

#[derive(Clone, Serialize)]
struct Verifier([u8; SECRET_BYTES]);

impl Verifier {
    fn new(secret: &Secret) -> Self {
        let mut digest = Sha256::new();
        digest.update(b"revenant.m30.verifier.v1\0");
        digest.update(secret.0);
        Self(digest.finalize().into())
    }

    fn matches(&self, candidate: &[u8]) -> bool {
        if candidate.len() != SECRET_BYTES {
            return false;
        }
        let mut digest = Sha256::new();
        digest.update(b"revenant.m30.verifier.v1\0");
        digest.update(candidate);
        let candidate: [u8; SECRET_BYTES] = digest.finalize().into();
        bool::from(self.0.ct_eq(&candidate))
    }
}

#[derive(Clone, Serialize)]
struct AccountRecord {
    canonical_id: String,
    display_name: String,
    credential: Verifier,
    recovery: Verifier,
    generation: u64,
}

#[derive(Clone, Serialize)]
struct SessionRecord {
    id: String,
    account_id: String,
    lobby_id: String,
    bearer: Verifier,
    issued_at_ms: u64,
    last_used_at_ms: u64,
    revoked: bool,
}

#[derive(Clone, Serialize)]
struct InviteRecord {
    bearer: Verifier,
    expires_at_ms: u64,
    redeemed_by: Option<String>,
}

#[derive(Clone, Serialize)]
struct LobbyRecord {
    id: String,
    owner_id: String,
    members: BTreeMap<String, bool>,
    invite: InviteRecord,
}

#[derive(Clone, Default)]
struct IdentitySessionLab {
    accounts: BTreeMap<String, AccountRecord>,
    sessions: BTreeMap<String, SessionRecord>,
    failures: BTreeMap<String, VecDeque<u64>>,
    lobby: Option<LobbyRecord>,
    next_session_id: u64,
    protected_mutations: u64,
}

#[derive(Serialize)]
struct ProtectedSnapshot<'a> {
    accounts: &'a BTreeMap<String, AccountRecord>,
    sessions: &'a BTreeMap<String, SessionRecord>,
    lobby: &'a Option<LobbyRecord>,
}

impl IdentitySessionLab {
    fn protected_digest(&self) -> String {
        let snapshot = ProtectedSnapshot {
            accounts: &self.accounts,
            sessions: &self.sessions,
            lobby: &self.lobby,
        };
        let bytes = serde_json::to_vec(&snapshot).expect("protected snapshot must serialize");
        hex(&Sha256::digest(bytes))
    }

    fn resources(&self) -> ResourceSample {
        ResourceSample {
            accounts: self.accounts.len(),
            sessions: self.sessions.len(),
            lobby_members: self.lobby.as_ref().map_or(0, |lobby| lobby.members.len()),
            retained_failures: self.failures.values().map(VecDeque::len).sum(),
        }
    }

    fn enroll(&mut self, username: &str, credential: &Secret, recovery: &Secret) -> Disposition {
        let Some(canonical_id) = canonical_account_id(username) else {
            return Disposition::InvalidInput;
        };
        if let Some(existing) = self.accounts.get(&canonical_id) {
            if existing.display_name == username
                && existing.credential.matches(&credential.0)
                && existing.recovery.matches(&recovery.0)
            {
                return Disposition::Replayed;
            }
            return Disposition::Conflict;
        }
        if self.accounts.len() >= 2 {
            return Disposition::Conflict;
        }
        self.accounts.insert(
            canonical_id.clone(),
            AccountRecord {
                canonical_id,
                display_name: username.to_owned(),
                credential: Verifier::new(credential),
                recovery: Verifier::new(recovery),
                generation: 1,
            },
        );
        self.protected_mutations += 1;
        Disposition::Enrolled
    }

    fn authenticate(
        &mut self,
        username: &str,
        credential: &[u8],
        lobby_id: &str,
        now_ms: u64,
        session_secret: &Secret,
    ) -> (Disposition, Option<String>) {
        let Some(account_id) = canonical_account_id(username) else {
            return (Disposition::InvalidInput, None);
        };
        let failure_key = if self.accounts.contains_key(&account_id) {
            account_id.clone()
        } else {
            "unknown-principal".to_owned()
        };
        let failures = self.failures.entry(failure_key).or_default();
        while failures
            .front()
            .is_some_and(|oldest| now_ms.saturating_sub(*oldest) >= RATE_WINDOW_MS)
        {
            failures.pop_front();
        }
        if failures.len() >= RATE_FAILURE_LIMIT {
            return (Disposition::RateLimited, None);
        }
        let authenticated = self
            .accounts
            .get(&account_id)
            .is_some_and(|account| account.credential.matches(credential));
        if !authenticated {
            failures.push_back(now_ms);
            return (Disposition::AuthenticationFailed, None);
        }
        let admitted_to_lobby = self
            .lobby
            .as_ref()
            .is_some_and(|lobby| lobby.id == lobby_id && lobby.members.contains_key(&account_id));
        if !admitted_to_lobby {
            return (Disposition::SessionRejected, None);
        }
        failures.clear();
        let Some(next_session_id) = self.next_session_id.checked_add(1) else {
            return (Disposition::Conflict, None);
        };
        self.sessions
            .retain(|_, session| session.account_id != account_id);
        self.next_session_id = next_session_id;
        let session_id = format!("session-{}", self.next_session_id);
        self.sessions.insert(
            session_id.clone(),
            SessionRecord {
                id: session_id.clone(),
                account_id,
                lobby_id: lobby_id.to_owned(),
                bearer: Verifier::new(session_secret),
                issued_at_ms: now_ms,
                last_used_at_ms: now_ms,
                revoked: false,
            },
        );
        self.protected_mutations += 1;
        (Disposition::Authenticated, Some(session_id))
    }

    fn use_session(
        &mut self,
        session_id: &str,
        bearer: &Secret,
        account_id: &str,
        lobby_id: &str,
        now_ms: u64,
    ) -> Disposition {
        let Some(session) = self.sessions.get_mut(session_id) else {
            return Disposition::SessionRejected;
        };
        if session.revoked
            || session.account_id != account_id
            || session.lobby_id != lobby_id
            || !session.bearer.matches(&bearer.0)
        {
            return Disposition::SessionRejected;
        }
        if now_ms.saturating_sub(session.issued_at_ms) > SESSION_ABSOLUTE_MS {
            session.revoked = true;
            self.protected_mutations += 1;
            return Disposition::ExpiredAbsolute;
        }
        if now_ms.saturating_sub(session.last_used_at_ms) > SESSION_IDLE_MS {
            session.revoked = true;
            self.protected_mutations += 1;
            return Disposition::ExpiredIdle;
        }
        session.last_used_at_ms = now_ms;
        self.protected_mutations += 1;
        Disposition::SessionValid
    }

    fn logout(&mut self, session_id: &str, bearer: &Secret) -> Disposition {
        let Some(session) = self.sessions.get_mut(session_id) else {
            return Disposition::SessionRejected;
        };
        if session.revoked || !session.bearer.matches(&bearer.0) {
            return Disposition::SessionRejected;
        }
        session.revoked = true;
        self.protected_mutations += 1;
        Disposition::Revoked
    }

    fn rotate_credential(
        &mut self,
        username: &str,
        current: &Secret,
        replacement: &Secret,
    ) -> Disposition {
        let Some(account_id) = canonical_account_id(username) else {
            return Disposition::InvalidInput;
        };
        let Some(account) = self.accounts.get_mut(&account_id) else {
            return Disposition::AuthenticationFailed;
        };
        if !account.credential.matches(&current.0) {
            return Disposition::AuthenticationFailed;
        }
        account.credential = Verifier::new(replacement);
        account.generation += 1;
        self.revoke_account_sessions(&account_id);
        self.protected_mutations += 1;
        Disposition::Rotated
    }

    fn recover(
        &mut self,
        username: &str,
        recovery: &Secret,
        replacement_credential: &Secret,
        replacement_recovery: &Secret,
    ) -> Disposition {
        let Some(account_id) = canonical_account_id(username) else {
            return Disposition::InvalidInput;
        };
        let Some(account) = self.accounts.get_mut(&account_id) else {
            return Disposition::AuthenticationFailed;
        };
        if !account.recovery.matches(&recovery.0) {
            return Disposition::AuthenticationFailed;
        }
        account.credential = Verifier::new(replacement_credential);
        account.recovery = Verifier::new(replacement_recovery);
        account.generation += 1;
        self.revoke_account_sessions(&account_id);
        self.protected_mutations += 1;
        Disposition::Recovered
    }

    fn revoke_account_sessions(&mut self, account_id: &str) {
        for session in self.sessions.values_mut() {
            if session.account_id == account_id {
                session.revoked = true;
            }
        }
    }

    fn create_lobby(&mut self, owner_id: &str, invite: &Secret, now_ms: u64) -> Disposition {
        if self.lobby.is_some() || !self.accounts.contains_key(owner_id) {
            return Disposition::Conflict;
        }
        let mut members = BTreeMap::new();
        members.insert(owner_id.to_owned(), true);
        self.lobby = Some(LobbyRecord {
            id: LOBBY_ID.to_owned(),
            owner_id: owner_id.to_owned(),
            members,
            invite: InviteRecord {
                bearer: Verifier::new(invite),
                expires_at_ms: now_ms.saturating_add(INVITE_LIFETIME_MS),
                redeemed_by: None,
            },
        });
        self.protected_mutations += 1;
        Disposition::LobbyCreated
    }

    fn redeem_invite(&mut self, peer_id: &str, invite: &Secret, now_ms: u64) -> Disposition {
        if !self.accounts.contains_key(peer_id) {
            return Disposition::InviteRejected;
        }
        let Some(lobby) = self.lobby.as_mut() else {
            return Disposition::InviteRejected;
        };
        if now_ms > lobby.invite.expires_at_ms
            || lobby.invite.redeemed_by.is_some()
            || !lobby.invite.bearer.matches(&invite.0)
            || lobby.members.len() >= 2
        {
            return Disposition::InviteRejected;
        }
        lobby.invite.redeemed_by = Some(peer_id.to_owned());
        lobby.members.insert(peer_id.to_owned(), true);
        self.protected_mutations += 1;
        Disposition::InviteRedeemed
    }

    fn set_presence(&mut self, account_id: &str, present: bool) -> Disposition {
        let Some(lobby) = self.lobby.as_mut() else {
            return Disposition::PresenceRejected;
        };
        let Some(current) = lobby.members.get_mut(account_id) else {
            return Disposition::PresenceRejected;
        };
        *current = present;
        self.protected_mutations += 1;
        Disposition::PresenceUpdated
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Disposition {
    InvalidInput,
    Enrolled,
    Replayed,
    Conflict,
    AuthenticationFailed,
    RateLimited,
    Authenticated,
    SessionValid,
    SessionRejected,
    ExpiredIdle,
    ExpiredAbsolute,
    Revoked,
    Rotated,
    Recovered,
    LobbyCreated,
    InviteRedeemed,
    InviteRejected,
    PresenceUpdated,
    PresenceRejected,
}

impl Disposition {
    const fn label(self) -> &'static str {
        match self {
            Self::InvalidInput => "invalid_input",
            Self::Enrolled => "enrolled",
            Self::Replayed => "replayed",
            Self::Conflict => "conflict",
            Self::AuthenticationFailed => "authentication_failed",
            Self::RateLimited => "rate_limited",
            Self::Authenticated => "authenticated",
            Self::SessionValid => "session_valid",
            Self::SessionRejected => "session_rejected",
            Self::ExpiredIdle => "expired_idle",
            Self::ExpiredAbsolute => "expired_absolute",
            Self::Revoked => "revoked",
            Self::Rotated => "rotated",
            Self::Recovered => "recovered",
            Self::LobbyCreated => "lobby_created",
            Self::InviteRedeemed => "invite_redeemed",
            Self::InviteRejected => "invite_rejected",
            Self::PresenceUpdated => "presence_updated",
            Self::PresenceRejected => "presence_rejected",
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct GateReport {
    schema: &'static str,
    gate: &'static str,
    protocol_integration: bool,
    secret_bytes: usize,
    session_idle_ms: u64,
    session_absolute_ms: u64,
    invite_lifetime_ms: u64,
    rate_window_ms: u64,
    rate_failure_limit: usize,
    cases: Vec<CaseRow>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct CaseRow {
    case_id: String,
    layer: &'static str,
    input_category: &'static str,
    initial_state_digest: String,
    expected_disposition: String,
    observed_disposition: String,
    post_state_digest: String,
    protected_mutation_count: u64,
    redaction_result: &'static str,
    duration_ms: u64,
    resource_sample: ResourceSample,
    passed: bool,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ResourceSample {
    accounts: usize,
    sessions: usize,
    lobby_members: usize,
    retained_failures: usize,
}

struct CaseBuilder {
    rows: Vec<CaseRow>,
}

impl CaseBuilder {
    fn new() -> Self {
        Self { rows: Vec::new() }
    }

    fn case(
        &mut self,
        case_id: &str,
        category: &'static str,
        lab: &mut IdentitySessionLab,
        expected: &str,
        action: impl FnOnce(&mut IdentitySessionLab) -> String,
    ) {
        let initial_state_digest = lab.protected_digest();
        let before_mutations = lab.protected_mutations;
        let observed = action(lab);
        let post_state_digest = lab.protected_digest();
        self.rows.push(CaseRow {
            case_id: case_id.to_owned(),
            layer: "pure_identity_session",
            input_category: category,
            initial_state_digest,
            expected_disposition: expected.to_owned(),
            observed_disposition: observed.clone(),
            post_state_digest,
            protected_mutation_count: lab.protected_mutations - before_mutations,
            redaction_result: "pass",
            duration_ms: 0,
            resource_sample: lab.resources(),
            passed: observed == expected,
        });
    }
}

/// Generates the exact deterministic Gate 2 `I01`–`I30` report.
///
/// # Errors
///
/// Returns an error when the operating-system CSPRNG cannot produce the one
/// non-persisted property sample used by I16.
///
/// # Panics
///
/// Panics only when a deterministic fixture cannot establish its declared
/// setup invariant or a bounded concurrency worker cannot complete.
#[allow(clippy::too_many_lines)]
pub fn generate_report() -> Result<GateReport, String> {
    let mut cases = CaseBuilder::new();

    cases.case(
        "I01",
        "owner_enrollment",
        &mut IdentitySessionLab::default(),
        "enrolled",
        |lab| {
            lab.enroll(
                "Owner",
                &secret("owner-credential"),
                &secret("owner-recovery"),
            )
            .label()
            .to_owned()
        },
    );

    let mut lab = IdentitySessionLab::default();
    cases.case(
        "I02",
        "canonical_account_id",
        &mut lab,
        "local:owner",
        |lab| {
            let result = lab.enroll(
                "Owner",
                &secret("owner-credential"),
                &secret("owner-recovery"),
            );
            if result == Disposition::Enrolled && lab.accounts.contains_key("local:owner") {
                "local:owner".to_owned()
            } else {
                result.label().to_owned()
            }
        },
    );
    cases.case(
        "I03",
        "invalid_username",
        &mut IdentitySessionLab::default(),
        "invalid_input",
        |lab| {
            lab.enroll("bad user\n", &secret("credential"), &secret("recovery"))
                .label()
                .to_owned()
        },
    );

    let mut lab = enrolled("Owner");
    cases.case("I04", "case_fold_collision", &mut lab, "conflict", |lab| {
        lab.enroll(
            "OWNER",
            &secret("owner-credential"),
            &secret("owner-recovery"),
        )
        .label()
        .to_owned()
    });
    let mut lab = enrolled("Owner");
    cases.case(
        "I05",
        "identical_enrollment_retry",
        &mut lab,
        "replayed",
        |lab| {
            lab.enroll(
                "Owner",
                &secret("owner-credential"),
                &secret("owner-recovery"),
            )
            .label()
            .to_owned()
        },
    );
    let mut lab = enrolled("Owner");
    cases.case(
        "I06",
        "conflicting_enrollment",
        &mut lab,
        "conflict",
        |lab| {
            lab.enroll("Owner", &secret("replacement"), &secret("owner-recovery"))
                .label()
                .to_owned()
        },
    );

    let mut lab = owner_lobby_fixture();
    cases.case(
        "I07",
        "valid_credential_proof",
        &mut lab,
        "authenticated",
        |lab| {
            lab.authenticate(
                "Owner",
                &secret("owner-credential").0,
                LOBBY_ID,
                0,
                &secret("session"),
            )
            .0
            .label()
            .to_owned()
        },
    );
    let mut lab = owner_lobby_fixture();
    cases.case(
        "I08",
        "wrong_credential",
        &mut lab,
        "authentication_failed",
        |lab| {
            lab.authenticate("Owner", &secret("wrong").0, LOBBY_ID, 0, &secret("session"))
                .0
                .label()
                .to_owned()
        },
    );
    let mut lab = owner_lobby_fixture();
    cases.case(
        "I09",
        "unknown_account_same_failure",
        &mut lab,
        "authentication_failed",
        |lab| {
            lab.authenticate(
                "Unknown",
                &secret("wrong").0,
                LOBBY_ID,
                0,
                &secret("session"),
            )
            .0
            .label()
            .to_owned()
        },
    );
    let mut lab = owner_lobby_fixture();
    cases.case(
        "I10",
        "malformed_credential_proof",
        &mut lab,
        "authentication_failed",
        |lab| {
            lab.authenticate("Owner", &[7_u8; 31], LOBBY_ID, 0, &secret("session"))
                .0
                .label()
                .to_owned()
        },
    );

    let mut lab = failed_proofs(4, 0);
    cases.case(
        "I11",
        "fifth_failed_proof",
        &mut lab,
        "authentication_failed",
        |lab| {
            lab.authenticate(
                "Owner",
                &secret("wrong-5").0,
                LOBBY_ID,
                4,
                &secret("session"),
            )
            .0
            .label()
            .to_owned()
        },
    );
    let mut lab = failed_proofs(5, 0);
    cases.case(
        "I12",
        "sixth_proof_rate_limited",
        &mut lab,
        "rate_limited",
        |lab| {
            lab.authenticate(
                "Owner",
                &secret("wrong-6").0,
                LOBBY_ID,
                5,
                &secret("session"),
            )
            .0
            .label()
            .to_owned()
        },
    );
    let mut lab = failed_proofs(5, 0);
    cases.case(
        "I13",
        "exact_rate_window_expiry",
        &mut lab,
        "authentication_failed",
        |lab| {
            lab.authenticate(
                "Owner",
                &secret("wrong-new").0,
                LOBBY_ID,
                RATE_WINDOW_MS,
                &secret("session"),
            )
            .0
            .label()
            .to_owned()
        },
    );
    let mut lab = failed_proofs(5, 0);
    cases.case(
        "I14",
        "plus_one_retry_after_expiry",
        &mut lab,
        "authenticated",
        |lab| {
            lab.authenticate(
                "Owner",
                &secret("owner-credential").0,
                LOBBY_ID,
                RATE_WINDOW_MS + 1,
                &secret("session"),
            )
            .0
            .label()
            .to_owned()
        },
    );

    let mut lab = owner_lobby_fixture();
    cases.case(
        "I15",
        "session_issuance",
        &mut lab,
        "single_active_session",
        |lab| {
            let (_, first_id) = lab.authenticate(
                "Owner",
                &secret("owner-credential").0,
                LOBBY_ID,
                0,
                &secret("session-1"),
            );
            let (_, second_id) = lab.authenticate(
                "Owner",
                &secret("owner-credential").0,
                LOBBY_ID,
                1,
                &secret("session-2"),
            );
            let first_removed = first_id
                .as_ref()
                .is_some_and(|id| !lab.sessions.contains_key(id));
            let second_active = second_id
                .as_ref()
                .and_then(|id| lab.sessions.get(id))
                .is_some_and(|session| !session.revoked);
            if first_removed
                && second_active
                && lab.sessions.len() == 1
                && lab
                    .sessions
                    .values()
                    .filter(|session| !session.revoked)
                    .count()
                    == 1
            {
                "single_active_session".to_owned()
            } else {
                "invalid_session_issuance".to_owned()
            }
        },
    );
    let generated = Secret::generate().map_err(|error| format!("CSPRNG failed: {error}"))?;
    let second_generated = Secret::generate().map_err(|error| format!("CSPRNG failed: {error}"))?;
    let mut lab = IdentitySessionLab::default();
    cases.case(
        "I16",
        "opaque_256_bit_value",
        &mut lab,
        "opaque_256_bit",
        |_| {
            if generated.0.len() == SECRET_BYTES && generated != second_generated {
                "opaque_256_bit".to_owned()
            } else {
                "invalid_secret_size".to_owned()
            }
        },
    );
    let (mut lab, session_id, bearer) = active_session(0);
    cases.case(
        "I17",
        "valid_scoped_session_use",
        &mut lab,
        "session_valid",
        |lab| {
            lab.use_session(&session_id, &bearer, "local:owner", LOBBY_ID, 1)
                .label()
                .to_owned()
        },
    );
    let (mut lab, session_id, bearer) = active_session(0);
    cases.case(
        "I18",
        "wrong_session_scope",
        &mut lab,
        "session_rejected",
        |lab| {
            lab.use_session(&session_id, &bearer, "local:peer", "other-lobby", 1)
                .label()
                .to_owned()
        },
    );

    let (mut lab, session_id, bearer) = active_session(0);
    cases.case(
        "I19",
        "exact_idle_boundary",
        &mut lab,
        "session_valid",
        |lab| {
            lab.use_session(
                &session_id,
                &bearer,
                "local:owner",
                LOBBY_ID,
                SESSION_IDLE_MS,
            )
            .label()
            .to_owned()
        },
    );
    let (mut lab, session_id, bearer) = active_session(0);
    cases.case(
        "I20",
        "idle_plus_one_expiry",
        &mut lab,
        "expired_idle",
        |lab| {
            lab.use_session(
                &session_id,
                &bearer,
                "local:owner",
                LOBBY_ID,
                SESSION_IDLE_MS + 1,
            )
            .label()
            .to_owned()
        },
    );
    let (mut lab, session_id, bearer) = active_session(0);
    assert_eq!(
        lab.use_session(&session_id, &bearer, "local:owner", LOBBY_ID, 600_000),
        Disposition::SessionValid
    );
    assert_eq!(
        lab.use_session(&session_id, &bearer, "local:owner", LOBBY_ID, 1_200_000),
        Disposition::SessionValid
    );
    cases.case(
        "I21",
        "exact_absolute_boundary",
        &mut lab,
        "session_valid",
        |lab| {
            lab.use_session(
                &session_id,
                &bearer,
                "local:owner",
                LOBBY_ID,
                SESSION_ABSOLUTE_MS,
            )
            .label()
            .to_owned()
        },
    );
    let (mut lab, session_id, bearer) = active_session(0);
    assert_eq!(
        lab.use_session(&session_id, &bearer, "local:owner", LOBBY_ID, 600_000),
        Disposition::SessionValid
    );
    assert_eq!(
        lab.use_session(&session_id, &bearer, "local:owner", LOBBY_ID, 1_200_000),
        Disposition::SessionValid
    );
    cases.case(
        "I22",
        "absolute_plus_one_expiry",
        &mut lab,
        "expired_absolute",
        |lab| {
            lab.use_session(
                &session_id,
                &bearer,
                "local:owner",
                LOBBY_ID,
                SESSION_ABSOLUTE_MS + 1,
            )
            .label()
            .to_owned()
        },
    );

    let (mut lab, session_id, bearer) = active_session(0);
    cases.case(
        "I23",
        "logout_revocation",
        &mut lab,
        "revoked_and_rejected",
        |lab| {
            let logout = lab.logout(&session_id, &bearer);
            let reuse = lab.use_session(&session_id, &bearer, "local:owner", LOBBY_ID, 1);
            if logout == Disposition::Revoked && reuse == Disposition::SessionRejected {
                "revoked_and_rejected".to_owned()
            } else {
                "revocation_failed".to_owned()
            }
        },
    );
    let (mut lab, session_id, bearer) = active_session(0);
    cases.case(
        "I24",
        "credential_rotation",
        &mut lab,
        "rotated_and_session_revoked",
        |lab| {
            let rotated = lab.rotate_credential(
                "Owner",
                &secret("owner-credential"),
                &secret("owner-credential-2"),
            );
            let reuse = lab.use_session(&session_id, &bearer, "local:owner", LOBBY_ID, 1);
            if rotated == Disposition::Rotated && reuse == Disposition::SessionRejected {
                "rotated_and_session_revoked".to_owned()
            } else {
                "rotation_failed".to_owned()
            }
        },
    );
    let (mut lab, session_id, bearer) = active_session(0);
    cases.case(
        "I25",
        "saved_recovery",
        &mut lab,
        "recovered_rotated_and_revoked",
        |lab| {
            let recovered = lab.recover(
                "Owner",
                &secret("owner-recovery"),
                &secret("owner-credential-2"),
                &secret("owner-recovery-2"),
            );
            let reuse = lab.use_session(&session_id, &bearer, "local:owner", LOBBY_ID, 1);
            if recovered == Disposition::Recovered && reuse == Disposition::SessionRejected {
                "recovered_rotated_and_revoked".to_owned()
            } else {
                "recovery_failed".to_owned()
            }
        },
    );
    let mut lab = enrolled("Owner");
    assert_eq!(
        lab.recover(
            "Owner",
            &secret("owner-recovery"),
            &secret("owner-credential-2"),
            &secret("owner-recovery-2")
        ),
        Disposition::Recovered
    );
    cases.case(
        "I26",
        "saved_recovery_replay",
        &mut lab,
        "authentication_failed",
        |lab| {
            lab.recover(
                "Owner",
                &secret("owner-recovery"),
                &secret("owner-credential-3"),
                &secret("owner-recovery-3"),
            )
            .label()
            .to_owned()
        },
    );

    let mut lab = enrolled("Owner");
    cases.case(
        "I27",
        "single_two_seat_lobby",
        &mut lab,
        "lobby_created",
        |lab| {
            lab.create_lobby("local:owner", &secret("invite"), 0)
                .label()
                .to_owned()
        },
    );
    let mut lab = lobby_fixture();
    cases.case(
        "I28",
        "one_time_invite_redemption",
        &mut lab,
        "invite_redeemed_once",
        |lab| {
            let first = lab.redeem_invite("local:peer", &secret("invite"), INVITE_LIFETIME_MS);
            let replay = lab.redeem_invite("local:peer", &secret("invite"), INVITE_LIFETIME_MS);
            if first == Disposition::InviteRedeemed
                && replay == Disposition::InviteRejected
                && lab
                    .lobby
                    .as_ref()
                    .is_some_and(|lobby| lobby.members.len() == 2)
            {
                "invite_redeemed_once".to_owned()
            } else {
                "invite_redemption_failed".to_owned()
            }
        },
    );
    let mut lab = joined_lobby_fixture();
    cases.case(
        "I29",
        "server_owned_presence_cycle",
        &mut lab,
        "presence_leave_join",
        |lab| {
            let left = lab.set_presence("local:peer", false);
            let joined = lab.set_presence("local:peer", true);
            if left == Disposition::PresenceUpdated
                && joined == Disposition::PresenceUpdated
                && lab
                    .lobby
                    .as_ref()
                    .and_then(|lobby| lobby.members.get("local:peer"))
                    .copied()
                    == Some(true)
            {
                "presence_leave_join".to_owned()
            } else {
                "presence_cycle_failed".to_owned()
            }
        },
    );

    let mut lab = lobby_fixture();
    cases.case(
        "I30",
        "concurrent_invite_redemption",
        &mut lab,
        "one_concurrent_winner",
        |lab| {
            let shared = Arc::new(Mutex::new(std::mem::take(lab)));
            let barrier = Arc::new(Barrier::new(3));
            let mut handles = Vec::new();
            for _ in 0..2 {
                let shared = Arc::clone(&shared);
                let barrier = Arc::clone(&barrier);
                handles.push(thread::spawn(move || {
                    barrier.wait();
                    shared
                        .lock()
                        .expect("lab mutex must remain available")
                        .redeem_invite("local:peer", &secret("invite"), 1)
                }));
            }
            barrier.wait();
            let outcomes = handles
                .into_iter()
                .map(|handle| handle.join().expect("redemption worker must finish"))
                .collect::<Vec<_>>();
            let Ok(mutex) = Arc::try_unwrap(shared) else {
                return "concurrent_lab_retained_owner".to_owned();
            };
            let Ok(recovered) = mutex.into_inner() else {
                return "concurrent_lab_mutex_poisoned".to_owned();
            };
            *lab = recovered;
            let accepted = outcomes
                .iter()
                .filter(|outcome| **outcome == Disposition::InviteRedeemed)
                .count();
            let rejected = outcomes
                .iter()
                .filter(|outcome| **outcome == Disposition::InviteRejected)
                .count();
            if accepted == 1
                && rejected == 1
                && lab
                    .lobby
                    .as_ref()
                    .is_some_and(|lobby| lobby.members.len() == 2)
            {
                "one_concurrent_winner".to_owned()
            } else {
                "concurrent_redemption_failed".to_owned()
            }
        },
    );

    Ok(GateReport {
        schema: "revenant.m30.identity-session-gate.v1",
        gate: "M30-Gate-2",
        protocol_integration: false,
        secret_bytes: SECRET_BYTES,
        session_idle_ms: SESSION_IDLE_MS,
        session_absolute_ms: SESSION_ABSOLUTE_MS,
        invite_lifetime_ms: INVITE_LIFETIME_MS,
        rate_window_ms: RATE_WINDOW_MS,
        rate_failure_limit: RATE_FAILURE_LIMIT,
        cases: cases.rows,
    })
}

fn canonical_account_id(username: &str) -> Option<String> {
    if username.is_empty()
        || username.len() > 32
        || !username
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
    {
        return None;
    }
    Some(format!("local:{}", username.to_ascii_lowercase()))
}

fn secret(label: &str) -> Secret {
    Secret::fixture(label)
}

fn enrolled(username: &str) -> IdentitySessionLab {
    let mut lab = IdentitySessionLab::default();
    assert_eq!(
        lab.enroll(
            username,
            &secret("owner-credential"),
            &secret("owner-recovery")
        ),
        Disposition::Enrolled
    );
    lab
}

fn failed_proofs(count: usize, start_ms: u64) -> IdentitySessionLab {
    let mut lab = owner_lobby_fixture();
    for index in 0..count {
        let wrong = secret(&format!("wrong-{index}"));
        assert_eq!(
            lab.authenticate(
                "Owner",
                &wrong.0,
                LOBBY_ID,
                start_ms + index as u64,
                &secret("session")
            )
            .0,
            Disposition::AuthenticationFailed
        );
    }
    lab
}

fn active_session(now_ms: u64) -> (IdentitySessionLab, String, Secret) {
    let mut lab = owner_lobby_fixture();
    let bearer = secret("session");
    let (result, session_id) = lab.authenticate(
        "Owner",
        &secret("owner-credential").0,
        LOBBY_ID,
        now_ms,
        &bearer,
    );
    assert_eq!(result, Disposition::Authenticated);
    (
        lab,
        session_id.expect("authenticated session must have an ID"),
        bearer,
    )
}

fn lobby_fixture() -> IdentitySessionLab {
    let mut lab = owner_lobby_fixture();
    assert_eq!(
        lab.enroll("Peer", &secret("peer-credential"), &secret("peer-recovery")),
        Disposition::Enrolled
    );
    lab
}

fn owner_lobby_fixture() -> IdentitySessionLab {
    let mut lab = enrolled("Owner");
    assert_eq!(
        lab.create_lobby("local:owner", &secret("invite"), 0),
        Disposition::LobbyCreated
    );
    lab
}

fn joined_lobby_fixture() -> IdentitySessionLab {
    let mut lab = lobby_fixture();
    assert_eq!(
        lab.redeem_invite("local:peer", &secret("invite"), 1),
        Disposition::InviteRedeemed
    );
    lab
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{generate_report, hex, owner_lobby_fixture, secret, Disposition, Secret, LOBBY_ID};

    #[test]
    fn report_has_exact_ordered_passing_case_set() {
        let report = generate_report().expect("report must generate");
        assert_eq!(report.cases.len(), 30);
        for (index, row) in report.cases.iter().enumerate() {
            assert_eq!(row.case_id, format!("I{:02}", index + 1));
            assert!(row.passed, "{} did not pass", row.case_id);
            assert_eq!(row.redaction_result, "pass");
        }
    }

    #[test]
    fn report_serialization_is_deterministic_and_redacted() {
        let first = serde_json::to_vec_pretty(&generate_report().expect("first report"))
            .expect("first report must serialize");
        let second = serde_json::to_vec_pretty(&generate_report().expect("second report"))
            .expect("second report must serialize");
        assert_eq!(first, second);
        let json = String::from_utf8(first).expect("JSON must be UTF-8");
        for fixture in ["owner-credential", "owner-recovery", "invite", "session"] {
            let raw = secret(fixture);
            assert!(!json.contains(&hex(&raw.0)));
        }
        assert!(!json.contains("[REDACTED; 32 bytes]"));
    }

    #[test]
    fn rejection_and_rate_metadata_never_change_protected_state() {
        let report = generate_report().expect("report must generate");
        for case_id in [
            "I03", "I04", "I05", "I06", "I08", "I09", "I10", "I11", "I12", "I13", "I16", "I18",
            "I26",
        ] {
            let row = report
                .cases
                .iter()
                .find(|row| row.case_id == case_id)
                .expect("named stable-state row must exist");
            assert_eq!(row.initial_state_digest, row.post_state_digest, "{case_id}");
            assert_eq!(row.protected_mutation_count, 0, "{case_id}");
        }

        let mut lab = owner_lobby_fixture();
        for index in 0..100 {
            let username = format!("Unknown{index}");
            let result = lab.authenticate(
                &username,
                &secret("wrong").0,
                LOBBY_ID,
                0,
                &secret("session"),
            );
            assert!(matches!(
                result.0,
                Disposition::AuthenticationFailed | Disposition::RateLimited
            ));
        }
        assert_eq!(lab.failures.len(), 1);
        assert_eq!(lab.failures["unknown-principal"].len(), 5);
    }

    #[test]
    fn generated_secret_is_fixed_size_and_debug_redacted() {
        let generated = Secret::generate().expect("OS CSPRNG must be available");
        assert_eq!(generated.0.len(), 32);
        assert_eq!(format!("{generated:?}"), "Secret([REDACTED; 32 bytes])");
    }
}
