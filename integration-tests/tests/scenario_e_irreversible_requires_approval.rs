//! Acceptance scenario E (Section 30): irreversible action (e.g. delete
//! resource) -> compensation UNSUPPORTED -> policy requires explicit
//! approval, regardless of the rest of the policy configuration.

use forge_core::{CompensationSupport, RiskLevel};
use forge_policy::{Decision, Policy};

#[test]
fn delete_with_unsupported_compensation_forces_approval_even_if_policy_is_lax() {
    let mut policy = Policy::default();
    policy.require_approval = false; // pretend the rest of policy is permissive

    let decision = policy.execution_decision(RiskLevel::Critical, CompensationSupport::Unsupported);
    assert_eq!(decision, Decision::RequireApproval);
}

#[test]
fn low_risk_supported_compensation_does_not_force_approval_when_policy_allows() {
    let mut policy = Policy::default();
    policy.require_approval = false;

    let decision = policy.execution_decision(RiskLevel::Low, CompensationSupport::Supported);
    assert_eq!(decision, Decision::Allow);
}
