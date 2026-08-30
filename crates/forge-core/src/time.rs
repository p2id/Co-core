use serde::{Deserialize, Serialize};
use std::fmt;

/// RFC3339 UTC timestamp. Wrapping `chrono::DateTime<Utc>` so every crate
/// serializes timestamps identically (matches the `date-time` format used
/// throughout `schemas/*.json`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp(chrono::DateTime<chrono::Utc>);

impl Timestamp {
    pub fn now() -> Self {
        Self(chrono::Utc::now())
    }

    pub fn from_rfc3339(s: &str) -> Result<Self, crate::ForgeError> {
        chrono::DateTime::parse_from_rfc3339(s)
            .map(|dt| Self(dt.with_timezone(&chrono::Utc)))
            .map_err(|e| crate::ForgeError::InvalidTimestamp(e.to_string()))
    }

    pub fn to_rfc3339(&self) -> String {
        self.0.to_rfc3339()
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_rfc3339())
    }
}
