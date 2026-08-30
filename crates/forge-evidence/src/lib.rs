//! Evidence (Section 18). Immutable once created (ADR 0003) — same
//! private-fields-plus-getters pattern as `forge_change::Change`, and the
//! storage layer (`forge-storage`) additionally never issues an `UPDATE`
//! against the `evidence` table.

use forge_core::{ActionId, ChangeId, EvidenceId, ExecutionId, PlanId, ResourceId, Timestamp};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceType {
    Observation,
    PlanCreation,
    Approval,
    Execution,
    CommandResult,
    StateTransition,
    Verification,
    Compensation,
    PolicyDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceResult {
    Passed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    id: EvidenceId,
    evidence_type: EvidenceType,
    timestamp: Timestamp,
    resource: Option<ResourceId>,
    change: Option<ChangeId>,
    plan: Option<PlanId>,
    execution: Option<ExecutionId>,
    action: Option<ActionId>,
    expected: Option<Value>,
    observed: Option<Value>,
    result: Option<EvidenceResult>,
}

#[derive(Default)]
pub struct EvidenceBuilder {
    resource: Option<ResourceId>,
    change: Option<ChangeId>,
    plan: Option<PlanId>,
    execution: Option<ExecutionId>,
    action: Option<ActionId>,
    expected: Option<Value>,
    observed: Option<Value>,
    result: Option<EvidenceResult>,
}

impl EvidenceBuilder {
    pub fn resource(mut self, r: ResourceId) -> Self {
        self.resource = Some(r);
        self
    }
    pub fn change(mut self, c: ChangeId) -> Self {
        self.change = Some(c);
        self
    }
    pub fn plan(mut self, p: PlanId) -> Self {
        self.plan = Some(p);
        self
    }
    pub fn execution(mut self, e: ExecutionId) -> Self {
        self.execution = Some(e);
        self
    }
    pub fn action(mut self, a: ActionId) -> Self {
        self.action = Some(a);
        self
    }
    pub fn expected(mut self, v: Value) -> Self {
        self.expected = Some(v);
        self
    }
    pub fn observed(mut self, v: Value) -> Self {
        self.observed = Some(v);
        self
    }
    pub fn result(mut self, r: EvidenceResult) -> Self {
        self.result = Some(r);
        self
    }

    /// Finalize into an immutable `Evidence`. This is the only way to
    /// produce one — there are no setters on `Evidence` itself.
    pub fn build(self, evidence_type: EvidenceType) -> Evidence {
        Evidence {
            id: EvidenceId::new(),
            evidence_type,
            timestamp: Timestamp::now(),
            resource: self.resource,
            change: self.change,
            plan: self.plan,
            execution: self.execution,
            action: self.action,
            expected: self.expected,
            observed: self.observed,
            result: self.result,
        }
    }
}

impl Evidence {
    pub fn builder() -> EvidenceBuilder {
        EvidenceBuilder::default()
    }
    pub fn id(&self) -> &EvidenceId {
        &self.id
    }
    pub fn evidence_type(&self) -> EvidenceType {
        self.evidence_type
    }
    pub fn change(&self) -> Option<&ChangeId> {
        self.change.as_ref()
    }
    pub fn plan(&self) -> Option<&PlanId> {
        self.plan.as_ref()
    }
    pub fn result(&self) -> Option<EvidenceResult> {
        self.result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_builder_produces_queryable_by_change() {
        let change = ChangeId::new();
        let ev = Evidence::builder()
            .change(change.clone())
            .result(EvidenceResult::Passed)
            .build(EvidenceType::Verification);

        assert_eq!(ev.change(), Some(&change));
        assert_eq!(ev.result(), Some(EvidenceResult::Passed));
    }
}
