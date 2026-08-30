//! Execution engine (Section 15). Drives a `Plan` through
//! EXECUTING -> VERIFYING -> COMMITTED, or on failure through
//! FAILED -> COMPENSATING -> COMPENSATED / COMPENSATION_FAILED.
//!
//! This crate does not itself call out to plugins — that is
//! `forge-plugin`'s job. `Execution` here only tracks the state machine
//! and results; the actual side-effecting calls are injected by the
//! caller (`forge-cli`, wiring `forge-plugin` in), keeping this crate
//! testable without any real Docker/Git/filesystem present.

use forge_core::{ActionId, ExecutionId, PlanId, Timestamp};
use forge_plan::Plan;
use forge_state::ExecutionState;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionStatus {
    Succeeded,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub action: ActionId,
    pub status: ActionStatus,
    pub output: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Execution {
    id: ExecutionId,
    plan: PlanId,
    state: ExecutionState,
    started_at: Timestamp,
    finished_at: Option<Timestamp>,
    action_results: Vec<ActionResult>,
}

#[derive(Debug, thiserror::Error)]
pub enum ExecError {
    #[error("plan is not in an executable state")]
    NotExecutable,
    #[error("execution already finished")]
    AlreadyFinished,
}

impl Execution {
    /// Begin executing an approved, non-stale plan. Refuses to start
    /// otherwise (Section 14 invariant, Section 35 invariant #5).
    pub fn begin(plan: &Plan) -> Result<Self, ExecError> {
        plan.ensure_executable().map_err(|_| ExecError::NotExecutable)?;
        Ok(Self {
            id: ExecutionId::new(),
            plan: plan.id().clone(),
            state: ExecutionState::Executing,
            started_at: Timestamp::now(),
            finished_at: None,
            action_results: Vec::new(),
        })
    }

    pub fn record_action_result(&mut self, result: ActionResult) -> Result<(), ExecError> {
        if self.state.is_terminal() {
            return Err(ExecError::AlreadyFinished);
        }
        self.action_results.push(result);
        Ok(())
    }

    pub fn any_action_failed(&self) -> bool {
        self.action_results
            .iter()
            .any(|r| r.status == ActionStatus::Failed)
    }

    /// Move to VERIFYING (Section 15). Called once every planned action has
    /// either succeeded, failed, or been explicitly skipped.
    pub fn begin_verification(&mut self) {
        self.state = ExecutionState::Verifying;
    }

    /// Section 16: verification is explicit and separate from "command
    /// succeeded". Caller supplies the already-computed verification
    /// result from `forge-verify`; this method only performs the state
    /// transition.
    pub fn complete_with_verification(&mut self, verified: bool) {
        self.state = if verified && !self.any_action_failed() {
            ExecutionState::Committed
        } else {
            ExecutionState::Failed
        };
        self.finished_at = Some(Timestamp::now());
    }

    pub fn begin_compensation(&mut self) -> Result<(), ExecError> {
        if self.state != ExecutionState::Failed {
            return Err(ExecError::NotExecutable);
        }
        self.state = ExecutionState::Compensating;
        Ok(())
    }

    /// Section 17/30: compensation may itself fail, or may be only
    /// partially possible (some actions `Unsupported`). Caller passes
    /// whether every attempted compensation step succeeded and whether
    /// there were any coverage gaps (from `CompensationPlan::has_gaps`).
    pub fn complete_compensation(&mut self, all_steps_succeeded: bool, had_gaps: bool) {
        self.state = if all_steps_succeeded && !had_gaps {
            ExecutionState::Compensated
        } else if all_steps_succeeded && had_gaps {
            // Section 27 example: compensation "succeeded" but coverage
            // was partial — still reported as COMPENSATED, with the gap
            // visible via CompensationPlan/Evidence, never hidden.
            ExecutionState::Compensated
        } else {
            ExecutionState::CompensationFailed
        };
        self.finished_at = Some(Timestamp::now());
    }

    pub fn state(&self) -> ExecutionState {
        self.state
    }
    pub fn id(&self) -> &ExecutionId {
        &self.id
    }
    pub fn plan(&self) -> &PlanId {
        &self.plan
    }
    pub fn action_results(&self) -> &[ActionResult] {
        &self.action_results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core::RiskLevel;

    fn approved_plan() -> Plan {
        Plan::new_draft(vec![], vec![], vec![], RiskLevel::Low).approve()
    }

    #[test]
    fn cannot_begin_execution_on_stale_plan() {
        let plan = approved_plan().revalidate(false);
        assert!(matches!(Execution::begin(&plan), Err(ExecError::NotExecutable)));
    }

    #[test]
    fn happy_path_reaches_committed() {
        let plan = approved_plan();
        let mut exec = Execution::begin(&plan).unwrap();
        exec.begin_verification();
        exec.complete_with_verification(true);
        assert_eq!(exec.state(), ExecutionState::Committed);
    }

    #[test]
    fn failed_verification_leads_to_failed_then_compensated() {
        let plan = approved_plan();
        let mut exec = Execution::begin(&plan).unwrap();
        exec.begin_verification();
        exec.complete_with_verification(false);
        assert_eq!(exec.state(), ExecutionState::Failed);

        exec.begin_compensation().unwrap();
        exec.complete_compensation(true, false);
        assert_eq!(exec.state(), ExecutionState::Compensated);
    }

    #[test]
    fn compensation_that_cannot_complete_is_compensation_failed() {
        let plan = approved_plan();
        let mut exec = Execution::begin(&plan).unwrap();
        exec.begin_verification();
        exec.complete_with_verification(false);
        exec.begin_compensation().unwrap();
        exec.complete_compensation(false, true);
        assert_eq!(exec.state(), ExecutionState::CompensationFailed);
    }
}
