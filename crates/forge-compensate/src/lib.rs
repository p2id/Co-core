//! Compensation (Section 17, ADR 0004). This is explicitly NOT "rollback":
//! see ADR 0004 for why. A `CompensationPlan` only ever contains actions
//! whose `CompensationSupport` is `Supported` or `Partial` — an
//! `Unsupported` action is recorded as a gap, never silently skipped.

use forge_core::{ActionId, CompensationSupport};
use forge_plan::Action;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompensationStep {
    pub original_action: ActionId,
    pub support: CompensationSupport,
    /// Human-readable description of the inverse operation, e.g.
    /// "restore previous image api:v1.4". `None` when support is
    /// `Unsupported`.
    pub inverse_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompensationPlan {
    pub steps: Vec<CompensationStep>,
}

impl CompensationPlan {
    /// Build a compensation plan from the actions that actually executed,
    /// in reverse order (last action compensated first). Actions with
    /// `Unsupported` compensation are still recorded as an explicit gap —
    /// never omitted, so the operator can see exactly what could not be
    /// undone (Section 17: "The Core must accurately expose").
    pub fn from_executed_actions(actions: &[Action]) -> Self {
        let steps = actions
            .iter()
            .rev()
            .map(|action| CompensationStep {
                original_action: action.id.clone(),
                support: action.compensation,
                inverse_description: match action.compensation {
                    CompensationSupport::Unsupported => None,
                    _ => Some(format!("compensate({})", action.kind)),
                },
            })
            .collect();
        Self { steps }
    }

    pub fn has_gaps(&self) -> bool {
        self.steps
            .iter()
            .any(|s| s.support == CompensationSupport::Unsupported)
    }

    pub fn fully_supported(&self) -> bool {
        self.steps
            .iter()
            .all(|s| s.support == CompensationSupport::Supported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core::ChangeId;

    fn action(compensation: CompensationSupport) -> Action {
        Action {
            id: forge_core::ActionId::new(),
            change: ChangeId::new(),
            kind: "docker.update_image".into(),
            preconditions: vec![],
            compensation,
        }
    }

    #[test]
    fn unsupported_action_is_recorded_as_gap_not_skipped() {
        let plan = CompensationPlan::from_executed_actions(&[
            action(CompensationSupport::Supported),
            action(CompensationSupport::Unsupported),
        ]);
        assert_eq!(plan.steps.len(), 2);
        assert!(plan.has_gaps());
        assert!(!plan.fully_supported());
    }

    #[test]
    fn compensation_order_is_reverse_of_execution() {
        let a1 = action(CompensationSupport::Supported);
        let a2 = action(CompensationSupport::Supported);
        let a1_id = a1.id.clone();
        let plan = CompensationPlan::from_executed_actions(&[a1, a2]);
        // last-executed (a2) compensated first
        assert_ne!(plan.steps[0].original_action, a1_id);
        assert_eq!(plan.steps[1].original_action, a1_id);
    }
}
