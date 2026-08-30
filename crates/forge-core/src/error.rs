use thiserror::Error;

#[derive(Debug, Error)]
pub enum ForgeError {
    #[error("invalid id: expected prefix '{expected_prefix}', got '{got}'")]
    InvalidId { expected_prefix: String, got: String },

    #[error("invalid resource id (expected provider://kind/id): '{0}'")]
    InvalidResourceId(String),

    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(String),

    #[error("plan is immutable once approved: {0}")]
    PlanImmutable(String),

    #[error("plan is stale: expected {expected}, observed {observed}")]
    PlanStale { expected: String, observed: String },

    #[error("capability denied: {0}")]
    CapabilityDenied(String),

    #[error("compensation unsupported for action: {0}")]
    CompensationUnsupported(String),

    #[error("storage error: {0}")]
    Storage(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
