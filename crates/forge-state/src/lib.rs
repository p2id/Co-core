//! The four state perspectives (Section 7) and the shared execution state
//! machine (Section 15) used by both Plan and Execution.

use forge_core::{ResourceId, Timestamp};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Section 7: never conflate these. In particular, `Perspective::Verified`
/// may only be produced by `forge-verify`, never assumed from
/// `Perspective::Planned` just because a command exited 0 (Section 16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Perspective {
    /// What actually exists, per the latest plugin observation.
    Observed,
    /// What the user/system intends.
    Desired,
    /// What Forge predicts will exist after executing a Plan.
    Planned,
    /// What has been proven, post-execution, by forge-verify.
    Verified,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub resource: ResourceId,
    pub perspective: Perspective,
    pub attributes: BTreeMap<String, Value>,
    pub recorded_at: Timestamp,
}

/// Section 15 execution state machine, shared by Plan and Execution.
/// Transitions are only ever performed by the crate that owns them
/// (`forge-plan` drives DRAFT..APPROVED..REVALIDATING; `forge-exec` drives
/// EXECUTING onward) — this enum is the shared vocabulary, not a place to
/// put transition logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionState {
    Draft,
    Planned,
    Approved,
    Revalidating,
    Executing,
    Verifying,
    Committed,
    Failed,
    Compensating,
    Compensated,
    CompensationFailed,
    /// A previously-approved plan whose preconditions no longer hold
    /// (Section 14). A stale plan can never transition to Executing.
    Stale,
}

impl ExecutionState {
    /// Whether a plan/execution in this state is allowed to move to
    /// `Executing`. Encodes the "never silently execute a stale plan"
    /// invariant (Section 14, Section 35 invariant #5) in one place so
    /// `forge-exec` and `forge-plan` cannot each drift with their own copy.
    pub fn can_execute(&self) -> bool {
        matches!(self, ExecutionState::Approved | ExecutionState::Revalidating)
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ExecutionState::Committed
                | ExecutionState::Compensated
                | ExecutionState::CompensationFailed
                | ExecutionState::Stale
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_plan_cannot_execute() {
        assert!(!ExecutionState::Stale.can_execute());
    }

    #[test]
    fn approved_plan_can_execute() {
        assert!(ExecutionState::Approved.can_execute());
    }

    #[test]
    fn draft_cannot_execute() {
        assert!(!ExecutionState::Draft.can_execute());
    }

    #[test]
    fn committed_is_terminal() {
        assert!(ExecutionState::Committed.is_terminal());
        assert!(!ExecutionState::Executing.is_terminal());
    }
}
