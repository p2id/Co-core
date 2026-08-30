//! Verification (Section 16). "Exit code 0 alone is insufficient" — this
//! crate deliberately does not accept a raw exit code as a verification
//! result; callers must supply an `expected` vs `observed` comparison.

use forge_core::{ResourceId, Timestamp};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationResult {
    Passed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verification {
    pub resource: ResourceId,
    pub expected: BTreeMap<String, Value>,
    pub observed: BTreeMap<String, Value>,
    pub checked_at: Timestamp,
    pub result: VerificationResult,
}

impl Verification {
    /// Compares expected vs. observed attribute-by-attribute. This is the
    /// only way to produce a `Verification` — there is no constructor that
    /// takes a boolean/exit-code shortcut, so a caller cannot claim
    /// "verified" without actually supplying both sides of the comparison.
    pub fn check(
        resource: ResourceId,
        expected: BTreeMap<String, Value>,
        observed: BTreeMap<String, Value>,
    ) -> Self {
        let passed = expected
            .iter()
            .all(|(k, v)| observed.get(k) == Some(v));

        Self {
            resource,
            expected,
            observed,
            checked_at: Timestamp::now(),
            result: if passed {
                VerificationResult::Passed
            } else {
                VerificationResult::Failed
            },
        }
    }

    pub fn passed(&self) -> bool {
        self.result == VerificationResult::Passed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verification_fails_when_expected_attribute_missing() {
        let mut expected = BTreeMap::new();
        expected.insert("health".to_string(), Value::String("healthy".into()));
        let observed = BTreeMap::new(); // healthcheck never reported

        let v = Verification::check(
            ResourceId::parse("docker://container/api").unwrap(),
            expected,
            observed,
        );
        assert!(!v.passed());
    }

    #[test]
    fn verification_passes_when_all_expected_match() {
        let mut expected = BTreeMap::new();
        expected.insert("image".to_string(), Value::String("api:v1.5".into()));
        let mut observed = BTreeMap::new();
        observed.insert("image".to_string(), Value::String("api:v1.5".into()));
        observed.insert("extra".to_string(), Value::String("ignored".into()));

        let v = Verification::check(
            ResourceId::parse("docker://container/api").unwrap(),
            expected,
            observed,
        );
        assert!(v.passed());
    }
}
