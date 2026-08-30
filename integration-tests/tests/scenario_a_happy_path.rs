//! Acceptance scenario A (Section 26/34): change image v1.4 -> v1.5,
//! detect -> plan -> revalidate -> execute -> verify -> commit.
//!
//! Uses the real Change/Plan/Exec/Verify types with an in-memory
//! stand-in instead of a live Docker daemon, since the docker plugin is
//! Phase 14 and not yet implemented — this test exercises the Core
//! pipeline the plugin will eventually be driven through.

use forge_change::{Change, ChangeSource, Operation};
use forge_core::{CompensationSupport, ResourceId, RiskLevel};
use forge_exec::Execution;
use forge_plan::{Action, ActionId, Plan};
use forge_verify::Verification;
use std::collections::BTreeMap;

#[test]
fn detect_plan_revalidate_execute_verify_commit() {
    let target = ResourceId::parse("docker://container/api").unwrap();

    // 1. detect
    let mut before = BTreeMap::new();
    before.insert("image".to_string(), serde_json::json!("api:v1.4"));
    let mut after = BTreeMap::new();
    after.insert("image".to_string(), serde_json::json!("api:v1.5"));

    let change = Change::new(
        target.clone(),
        Operation::Update,
        before,
        after.clone(),
        ChangeSource::User,
        Some("operator".into()),
        1.0,
    );

    // 2. plan
    let action = Action {
        id: ActionId::new(),
        change: change.id().clone(),
        kind: "docker.update_image".into(),
        preconditions: vec!["container api is running".into()],
        compensation: CompensationSupport::Supported,
    };
    let ordering = vec![action.id.clone()];
    let plan = Plan::new_draft(
        vec![change.id().clone()],
        vec![action],
        ordering,
        RiskLevel::Medium,
    );

    // 3. revalidate (state hasn't drifted -> still valid) then approve
    let plan = plan.approve().revalidate(true);
    assert!(plan.ensure_executable().is_ok());

    // 4. execute
    let mut exec = Execution::begin(&plan).unwrap();
    exec.record_action_result(forge_exec::ActionResult {
        action: plan.actions()[0].id.clone(),
        status: forge_exec::ActionStatus::Succeeded,
        output: serde_json::json!({ "image": "api:v1.5" }),
    })
    .unwrap();
    exec.begin_verification();

    // 5. verify
    let mut observed = BTreeMap::new();
    observed.insert("image".to_string(), serde_json::json!("api:v1.5"));
    let verification = Verification::check(target, after, observed);
    assert!(verification.passed());

    // 6. commit
    exec.complete_with_verification(verification.passed());
    assert_eq!(exec.state(), forge_state::ExecutionState::Committed);
}
