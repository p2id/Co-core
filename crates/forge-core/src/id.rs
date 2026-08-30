//! Strongly-typed identifiers so a `ChangeId` can never be silently passed
//! where a `PlanId` is expected — the type checker enforces the Core
//! invariant that these are distinct concepts (Section 5 of master context).

use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! typed_id {
    ($name:ident, $prefix:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            /// Generate a new random id with the correct prefix, e.g. `chg_a1b2c3`.
            pub fn new() -> Self {
                let uuid = uuid::Uuid::new_v4().simple().to_string();
                Self(format!("{}_{}", $prefix, &uuid[..12]))
            }

            /// Wrap an existing string as this id type, validating the prefix.
            pub fn parse(raw: impl Into<String>) -> Result<Self, crate::ForgeError> {
                let raw = raw.into();
                let expected_prefix = concat!($prefix, "_");
                if !raw.starts_with(expected_prefix) {
                    return Err(crate::ForgeError::InvalidId {
                        expected_prefix: expected_prefix.to_string(),
                        got: raw,
                    });
                }
                Ok(Self(raw))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<$name> for String {
            fn from(id: $name) -> String {
                id.0
            }
        }
    };
}

typed_id!(ChangeId, "chg");
typed_id!(PlanId, "plan");
typed_id!(ActionId, "act");
typed_id!(ExecutionId, "exec");
typed_id!(EvidenceId, "ev");

/// Resource identity is NOT a random id — it is the canonical
/// `provider://kind/id` string (Section 6). This wrapper only validates
/// the shape; it does not generate new values.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ResourceId(String);

impl ResourceId {
    pub fn parse(raw: impl Into<String>) -> Result<Self, crate::ForgeError> {
        let raw = raw.into();
        let has_scheme_sep = raw.find("://").map(|i| i > 0).unwrap_or(false);
        let after_scheme = raw.split("://").nth(1).unwrap_or("");
        let has_kind_and_id = after_scheme.splitn(2, '/').count() == 2
            && !after_scheme.splitn(2, '/').next().unwrap_or("").is_empty();

        if !has_scheme_sep || !has_kind_and_id {
            return Err(crate::ForgeError::InvalidResourceId(raw));
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn provider(&self) -> &str {
        self.0.split("://").next().unwrap_or("")
    }

    pub fn kind(&self) -> &str {
        self.0
            .split("://")
            .nth(1)
            .and_then(|rest| rest.split('/').next())
            .unwrap_or("")
    }
}

impl fmt::Display for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_id_parses_valid_shapes() {
        let r = ResourceId::parse("docker://container/api").unwrap();
        assert_eq!(r.provider(), "docker");
        assert_eq!(r.kind(), "container");
    }

    #[test]
    fn resource_id_rejects_missing_scheme() {
        assert!(ResourceId::parse("not-a-resource-id").is_err());
    }

    #[test]
    fn resource_id_rejects_missing_id_segment() {
        assert!(ResourceId::parse("docker://container").is_err());
    }

    #[test]
    fn change_id_roundtrip() {
        let id = ChangeId::new();
        let s = id.to_string();
        let parsed = ChangeId::parse(s.clone()).unwrap();
        assert_eq!(parsed.as_str(), s);
    }

    #[test]
    fn typed_ids_reject_wrong_prefix() {
        let plan_id_str = PlanId::new().to_string();
        assert!(ChangeId::parse(plan_id_str).is_err());
    }
}
