//! Policy and Capability (Sections 21, 23). Default is always DENY
//! (Section 23) — `Policy::default()` produces a policy that allows
//! nothing, so a missing/misconfigured policy fails closed, not open.

use forge_core::RiskLevel;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A named capability a plugin can request, e.g. "filesystem.write".
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Capability(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    Allow,
    Deny,
    RequireApproval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub environment: String,
    pub granted_capabilities: BTreeSet<Capability>,
    pub require_verification: bool,
    pub require_compensation: bool,
    pub require_approval: bool,
}

impl Default for Policy {
    /// Section 23: "Default: DENY." An empty/default policy grants nothing
    /// and requires verification, compensation, and approval for anything
    /// risky — the safe failure mode when no policy file is found.
    fn default() -> Self {
        Self {
            environment: "unknown".to_string(),
            granted_capabilities: BTreeSet::new(),
            require_verification: true,
            require_compensation: true,
            require_approval: true,
        }
    }
}

impl Policy {
    pub fn capability_decision(&self, cap: &Capability) -> Decision {
        if self.granted_capabilities.contains(cap) {
            Decision::Allow
        } else {
            Decision::Deny
        }
    }

    /// Section 30: an irreversible (CRITICAL risk + unsupported
    /// compensation) action always requires explicit approval, regardless
    /// of what the rest of the policy says.
    pub fn execution_decision(
        &self,
        risk: RiskLevel,
        compensation: forge_core::CompensationSupport,
    ) -> Decision {
        let irreversible_critical = risk == RiskLevel::Critical
            && compensation == forge_core::CompensationSupport::Unsupported;

        if irreversible_critical || self.require_approval {
            Decision::RequireApproval
        } else {
            Decision::Allow
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_denies_all_capabilities() {
        let p = Policy::default();
        assert_eq!(
            p.capability_decision(&Capability("filesystem.write".into())),
            Decision::Deny
        );
    }

    #[test]
    fn critical_unsupported_compensation_always_requires_approval() {
        let mut p = Policy::default();
        p.require_approval = false; // even if the rest of policy is lax...
        let decision = p.execution_decision(
            RiskLevel::Critical,
            forge_core::CompensationSupport::Unsupported,
        );
        // ...irreversible + critical still forces approval (Section 30).
        assert_eq!(decision, Decision::RequireApproval);
    }
}
