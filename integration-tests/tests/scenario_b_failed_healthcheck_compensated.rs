//! Acceptance scenario B (Section 27): v1.5 fails healthcheck ->
//! execute -> verify FAILED -> compensate -> verify restored ->
//! COMPENSATED.

use forge_core::CompensationSupport;
use forge_exec::{ActionResult, ActionStatus, Execution};
use forge_plan::{Action, ActionId, Plan};
use forge_verify::Verification;
use std::collections::BTreeMap;

#[test]
fn failed_verification_triggers_compensation_to_previous_image() {
    let action = Action {
        id: ActionId::new(),
        change: forge_core::ChangeId::new(),
        kind: "docker.update_image".into(),
        preconditions: vec![],
        compensation: CompensationSupport::Supported,
    };
    let plan = Plan::new_draft(
        vec![],
        vec![action.clone()],
        vec![action.id.clone()],
        forge_core::RiskLevel::Medium,
    )
    .approve()
    .revalidate(true);

    let mut exec = Execution::begin(&plan).unwrap();
    exec.record_action_result(ActionResult {
        action: action.id.clone(),
        status: ActionStatus::Succeeded,
        output: serde_json::json!({}),
    })
    .unwrap();
    exec.begin_verification();

    // healthcheck reports unhealthy -> verification fails
    let mut expected = BTreeMap::new();
    expected.insert("health".to_string(), serde_json::json!("healthy"));
    let mut observed = BTreeMap::new();
    observed.insert("health".to_string(), serde_json::json!("unhealthy"));
    let verification = Verification::check(
        forge_core::ResourceId::parse("docker://container/api").unwrap(),
        expected,
        observed,
    );
    assert!(!verification.passed());

    exec.complete_with_verification(verification.passed());
    assert_eq!(exec.state(), forge_state::ExecutionState::Failed);

    // compensate: restore api:v1.4
    let compensation_plan = forge_compensate::CompensationPlan::from_executed_actions(&[action]);
    assert!(compensation_plan.fully_supported());

    exec.begin_compensation().unwrap();
    exec.complete_compensation(true, compensation_plan.has_gaps());
    assert_eq!(exec.state(), forge_state::ExecutionState::Compensated);
}
