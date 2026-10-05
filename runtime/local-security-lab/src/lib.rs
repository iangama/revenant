#[cfg(feature = "m30-identity-session-lab")]
mod identity_session;

#[cfg(feature = "m30-identity-session-lab")]
pub use identity_session::{generate_report, GateReport, Secret};
