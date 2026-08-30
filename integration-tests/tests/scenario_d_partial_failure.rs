//! Acceptance scenario D (Section 29): partial execution failure ->
//! explicit partial state -> compensation if supported -> final state
//! accurately represented.

use forge_core::{ChangeId, CompensationSupport};
use forge_exec::{ActionResult, ActionStatus, Execution};
use forge_plan::{Action, ActionId, Plan};

fn action(compensation: CompensationSupport) -> Action {
    Action {
        id: ActionId::new(),
        change: ChangeId::new(),
        kind: "docker.update_image".into(),
        preconditions: vec![],
        compensation,
    }
}

#[test]
fn two_succeed_one_fails_one_skipped_then_partial_compensation() {
    let a1 = action(CompensationSupport::Supported);
    let a2 = action(CompensationSupport::Supported);
    let a3 = action(CompensationSupport::Supported);
    let a4 = action(CompensationSupport::Supported);

    let ordering = vec![a1.id.clone(), a2.id.clone(), a3.id.clone(), a4.id.clone()];
    let plan = Plan::new_draft(
        vec![],
        vec![a1.clone(), a2.clone(), a3.clone(), a4.clone()],
        ordering,
        forge_core::RiskLevel::High,
    )
    .approve()
    .revalidate(true);

    let mut exec = Execution::begin(&plan).unwrap();
    exec.record_action_result(ActionResult { action: a1.id.clone(), status: ActionStatus::Succeeded, output: serde_json::json!({}) }).unwrap();
    exec.record_action_result(ActionResult { action: a2.id.clone(), status: ActionStatus::Succeeded, output: serde_json::json!({}) }).unwrap();
    exec.record_action_result(ActionResult { action: a3.id.clone(), status: ActionStatus::Failed, output: serde_json::json!({"error": "timeout"}) }).unwrap();
    exec.record_action_result(ActionResult { action: a4.id.clone(), status: ActionStatus::Skipped, output: serde_json::json!({}) }).unwrap();

    assert!(exec.any_action_failed());

    let succeeded: Vec<_> = [a1, a2]
        .into_iter()
        .collect();
    let compensation_plan = forge_compensate::CompensationPlan::from_executed_actions(&succeeded);
    assert_eq!(compensation_plan.steps.len(), 2, "only completed actions are compensated");

    exec.begin_verification();
    exec.complete_with_verification(false); // any_action_failed() -> Failed regardless
    assert_eq!(exec.state(), forge_state::ExecutionState::Failed);

    exec.begin_compensation().unwrap();
    exec.complete_compensation(true, compensation_plan.has_gaps());
    assert_eq!(exec.state(), forge_state::ExecutionState::Compensated);

    let counts = exec.action_results().iter().fold((0, 0, 0), |mut acc, r| {
        match r.status {
            ActionStatus::Succeeded => acc.0 += 1,
            ActionStatus::Failed => acc.1 += 1,
            ActionStatus::Skipped => acc.2 += 1,
        }
        acc
    });
    assert_eq!(counts, (2, 1, 1), "completed=2 failed=1 skipped=1 must be explicit, not hidden");
}
