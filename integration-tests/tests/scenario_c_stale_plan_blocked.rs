//! Acceptance scenario C (Section 28): plan generated on api=v1.4,
//! system changes to v1.6 before apply -> STALE -> execution blocked.

use forge_exec::{ExecError, Execution};
use forge_plan::Plan;

#[test]
fn apply_on_drifted_state_is_blocked_as_stale() {
    let plan = Plan::new_draft(vec![], vec![], vec![], forge_core::RiskLevel::Low).approve();

    // Between plan creation and apply, another process changed
    // api v1.4 -> v1.6. The caller (forge-cli, in real usage) re-observes
    // current state, compares it to the Change's `before` snapshot, finds
    // a mismatch, and passes `still_valid = false` into revalidate.
    let observed_before_drifted = false;
    let plan = plan.revalidate(observed_before_drifted);

    assert!(plan.is_stale());
    let result = Execution::begin(&plan);
    assert!(matches!(result, Err(ExecError::NotExecutable)));
}
