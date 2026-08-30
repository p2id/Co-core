//! forge-core: shared primitives used by every other Forge crate.
//!
//! ARCHITECTURAL INVARIANT (ADR 0001): this crate and every other domain
//! crate in `crates/` must never depend on `forge-cli` or any AI/LLM crate.

pub mod id;
pub mod time;
pub mod error;

pub use error::ForgeError;
pub use id::{ActionId, ChangeId, EvidenceId, ExecutionId, PlanId, ResourceId};
pub use time::Timestamp;

/// Risk level, computed deterministically (ADR 0006) — never by an LLM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            RiskLevel::Low => "LOW",
            RiskLevel::Medium => "MEDIUM",
            RiskLevel::High => "HIGH",
            RiskLevel::Critical => "CRITICAL",
        };
        write!(f, "{s}")
    }
}

/// Whether a compensating (inverse-ish) operation exists for an action.
/// Never called "rollback" — see ADR 0004.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompensationSupport {
    Supported,
    Partial,
    Unsupported,
}
