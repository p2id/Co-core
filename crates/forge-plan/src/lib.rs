//! Action, Plan, and deterministic risk assessment (Sections 12, 13, 14,
//! 20; ADR 0006). Plan becomes immutable after approval (ADR 0003): the
//! type-state is enforced by `Plan::approve` consuming `self` and
//! returning a value whose mutating methods all fail.

// Re-exported (not just imported) so downstream crates can name the id
// types used in this crate's public API (e.g. `Action.id`) without also
// taking a direct dependency on forge-core just for that.
pub use forge_core::{ActionId, ChangeId, CompensationSupport, PlanId, RiskLevel, Timestamp};

use forge_state::ExecutionState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: ActionId,
    pub change: ChangeId,
    /// Plugin-defined operation kind, e.g. "docker.update_image".
    pub kind: String,
    pub preconditions: Vec<String>,
    pub compensation: CompensationSupport,
}

/// Deterministic risk factors (ADR 0006) — every field must come from
/// already-observed/declared data, never model output.
#[derive(Debug, Clone, Copy)]
pub struct RiskFactors {
    /// How many resources are transitively affected (Section: fundamental
    /// problem, item 5 / `forge-graph::affected_by`).
    pub blast_radius: usize,
    pub environment: Environment,
    pub privileged: bool,
    pub compensation: CompensationSupport,
    pub dependency_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Staging,
    Production,
}

/// ADR 0006: pure, deterministic function. No LLM call, ever, in v0.1.
pub fn assess_risk(factors: RiskFactors) -> RiskLevel {
    let mut score: i32 = 0;

    score += match factors.blast_radius {
        0..=1 => 0,
        2..=5 => 1,
        6..=15 => 2,
        _ => 3,
    };

    score += match factors.environment {
        Environment::Development => 0,
        Environment::Staging => 1,
        Environment::Production => 2,
    };

    if factors.privileged {
        score += 1;
    }

    score += match factors.compensation {
        CompensationSupport::Supported => 0,
        CompensationSupport::Partial => 1,
        CompensationSupport::Unsupported => 2,
    };

    score += match factors.dependency_count {
        0..=2 => 0,
        3..=8 => 1,
        _ => 2,
    };

    match score {
        0..=1 => RiskLevel::Low,
        2..=3 => RiskLevel::Medium,
        4..=6 => RiskLevel::High,
        _ => RiskLevel::Critical,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    id: PlanId,
    status: PlanStatusData,
    changes: Vec<ChangeId>,
    actions: Vec<Action>,
    ordering: Vec<ActionId>,
    constraints: Vec<String>,
    risk: RiskLevel,
    verification_strategy: Option<String>,
    compensation_strategy: Option<String>,
    created_at: Timestamp,
    approved_at: Option<Timestamp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum PlanStatusData {
    Draft,
    Planned,
    Approved,
    Stale,
}

#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    #[error("plan is immutable once approved")]
    Immutable,
    #[error("plan is stale and cannot execute")]
    Stale,
}

impl Plan {
    pub fn new_draft(
        changes: Vec<ChangeId>,
        actions: Vec<Action>,
        ordering: Vec<ActionId>,
        risk: RiskLevel,
    ) -> Self {
        Self {
            id: PlanId::new(),
            status: PlanStatusData::Draft,
            changes,
            actions,
            ordering,
            constraints: Vec::new(),
            risk,
            verification_strategy: None,
            compensation_strategy: None,
            created_at: Timestamp::now(),
            approved_at: None,
        }
    }

    pub fn id(&self) -> &PlanId {
        &self.id
    }

    pub fn risk(&self) -> RiskLevel {
        self.risk
    }

    pub fn actions(&self) -> &[Action] {
        &self.actions
    }

    pub fn ordering(&self) -> &[ActionId] {
        &self.ordering
    }

    pub fn is_approved(&self) -> bool {
        self.status == PlanStatusData::Approved
    }

    /// Add a constraint. Fails once the plan is approved (ADR 0003).
    pub fn add_constraint(&mut self, constraint: String) -> Result<(), PlanError> {
        if self.status == PlanStatusData::Approved {
            return Err(PlanError::Immutable);
        }
        self.constraints.push(constraint);
        Ok(())
    }

    /// Approve the plan. After this, `add_constraint` and any other
    /// mutation is rejected (ADR 0003) — approval is a one-way door.
    pub fn approve(mut self) -> Self {
        self.status = PlanStatusData::Approved;
        self.approved_at = Some(Timestamp::now());
        self
    }

    /// Section 14: revalidation. `still_valid` is supplied by the caller
    /// (e.g. by re-observing current state and comparing to the Change's
    /// `before` snapshot) — this function only encodes the state
    /// transition, never the comparison logic itself, keeping "what counts
    /// as valid" out of forge-plan's hands where it doesn't belong.
    pub fn revalidate(mut self, still_valid: bool) -> Self {
        if !still_valid {
            self.status = PlanStatusData::Stale;
        }
        self
    }

    pub fn is_stale(&self) -> bool {
        self.status == PlanStatusData::Stale
    }

    /// Section 14 invariant: never silently execute a stale plan.
    pub fn ensure_executable(&self) -> Result<(), PlanError> {
        match self.status {
            PlanStatusData::Stale => Err(PlanError::Stale),
            PlanStatusData::Approved => Ok(()),
            _ => Err(PlanError::Stale), // not yet approved is also non-executable
        }
    }

    pub fn to_execution_state(&self) -> ExecutionState {
        match self.status {
            PlanStatusData::Draft => ExecutionState::Draft,
            PlanStatusData::Planned => ExecutionState::Planned,
            PlanStatusData::Approved => ExecutionState::Approved,
            PlanStatusData::Stale => ExecutionState::Stale,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_risk_when_everything_favorable() {
        let risk = assess_risk(RiskFactors {
            blast_radius: 1,
            environment: Environment::Development,
            privileged: false,
            compensation: CompensationSupport::Supported,
            dependency_count: 0,
        });
        assert_eq!(risk, RiskLevel::Low);
    }

    #[test]
    fn critical_risk_when_everything_unfavorable() {
        let risk = assess_risk(RiskFactors {
            blast_radius: 50,
            environment: Environment::Production,
            privileged: true,
            compensation: CompensationSupport::Unsupported,
            dependency_count: 20,
        });
        assert_eq!(risk, RiskLevel::Critical);
    }

    #[test]
    fn approved_plan_rejects_further_mutation() {
        let plan = Plan::new_draft(vec![], vec![], vec![], RiskLevel::Low).approve();
        let mut plan = plan;
        assert!(matches!(
            plan.add_constraint("late".into()),
            Err(PlanError::Immutable)
        ));
    }

    #[test]
    fn stale_plan_cannot_be_executed() {
        let plan = Plan::new_draft(vec![], vec![], vec![], RiskLevel::Low)
            .approve()
            .revalidate(false);
        assert!(plan.is_stale());
        assert!(matches!(plan.ensure_executable(), Err(PlanError::Stale)));
    }

    #[test]
    fn revalidated_valid_plan_remains_executable() {
        let plan = Plan::new_draft(vec![], vec![], vec![], RiskLevel::Low)
            .approve()
            .revalidate(true);
        assert!(plan.ensure_executable().is_ok());
    }
}
