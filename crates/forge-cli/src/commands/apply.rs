//! `forge apply` (Section 24, Phase 7). Loads the latest APPROVED plan,
//! revalidates it against a fresh filesystem observation (Section 14 —
//! never silently execute a stale plan), executes each action, verifies
//! the result (Section 16), and commits or compensates (Section 17).

use forge_core::{CompensationSupport, ResourceId, Timestamp};
use forge_exec::{ActionResult, ActionStatus, Execution};
use forge_plan::{Action, ActionId, Plan};
use forge_plugin::Plugin;
use forge_plugin_filesystem::FilesystemPlugin;
use forge_verify::Verification;
use serde_json::Value;
use std::collections::BTreeMap;

fn to_btree(v: &Value) -> BTreeMap<String, Value> {
    v.as_object()
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default()
}

pub fn run() -> anyhow::Result<()> {
    let storage = super::open_storage()?;

    let plan_row = match storage
        .latest_approved_plan()
        .map_err(|e| anyhow::anyhow!(e.to_string()))?
    {
        Some(p) => p,
        None => {
            println!("No approved plan to apply. Run `forge plan --yes` first.");
            return Ok(());
        }
    };

    let action_ids: Vec<String> = serde_json::from_str(&plan_row.actions_json)?;
    let ordering: Vec<String> = serde_json::from_str(&plan_row.ordering_json)?;
    let change_ids: Vec<String> = serde_json::from_str(&plan_row.changes_json)?;
    let risk: forge_core::RiskLevel = match plan_row.risk.as_str() {
        "LOW" => forge_core::RiskLevel::Low,
        "MEDIUM" => forge_core::RiskLevel::Medium,
        "HIGH" => forge_core::RiskLevel::High,
        _ => forge_core::RiskLevel::Critical,
    };

    // Section 14: Revalidation. Re-observe each target and compare to the
    // Change's `after` snapshot recorded at plan time — if reality has
    // moved on since, the plan is STALE and execution is blocked.
    let fs_plugin = FilesystemPlugin::new(".");
    let mut still_valid = true;
    let mut loaded_actions: Vec<Action> = Vec::new();

    for change_id in &change_ids {
        let change_row = storage
            .get_change(change_id)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?
            .ok_or_else(|| anyhow::anyhow!("plan references unknown change {change_id}"))?;

        let expected_after: Value = serde_json::from_str(&change_row.after_json)?;
        let observations = fs_plugin
            .observe(&change_row.target)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        // Reconstruct a minimal "currently observed" object from the
        // plugin's Observation list for comparison purposes.
        let mut current = serde_json::Map::new();
        for obs in &observations {
            current.insert(obs.attribute.clone(), obs.value.clone());
        }
        let expected_size = expected_after.get("size_bytes");
        let current_size = current.get("size_bytes");
        if expected_after != Value::Null && expected_size.is_some() && expected_size != current_size {
            still_valid = false;
        }
    }

    for action_id in &action_ids {
        let row = storage
            .get_action(action_id)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?
            .ok_or_else(|| anyhow::anyhow!("plan references unknown action {action_id}"))?;
        let compensation = match row.compensation.as_str() {
            "supported" => CompensationSupport::Supported,
            "partial" => CompensationSupport::Partial,
            _ => CompensationSupport::Unsupported,
        };
        loaded_actions.push(Action {
            id: ActionId::parse(row.id)?,
            change: forge_core::ChangeId::parse(row.change_id)?,
            kind: row.kind,
            preconditions: serde_json::from_str(&row.preconditions_json)?,
            compensation,
        });
    }

    let plan_ordering: Vec<ActionId> = ordering
        .iter()
        .map(|s| ActionId::parse(s.clone()))
        .collect::<Result<_, _>>()?;
    let plan_change_ids: Vec<forge_core::ChangeId> = change_ids
        .iter()
        .map(|s| forge_core::ChangeId::parse(s.clone()))
        .collect::<Result<_, _>>()?;

    let plan = Plan::new_draft(plan_change_ids, loaded_actions.clone(), plan_ordering, risk)
        .approve()
        .revalidate(still_valid);

    if plan.is_stale() {
        storage
            .mark_plan_stale(&plan_row.id)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        println!("PLAN STALE");
        println!();
        println!("Observed state no longer matches what this plan expected.");
        println!("Execution blocked. Run `forge scan` then `forge plan --yes` again.");
        return Ok(());
    }

    let mut exec = Execution::begin(&plan).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let started_at = Timestamp::now().to_rfc3339();
    storage
        .insert_execution(exec.id().as_str(), &plan_row.id, "EXECUTING", &started_at)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    // Execute each action. "filesystem.acknowledge_change" (the only kind
    // `forge plan` currently produces) has nothing to actively do — the
    // underlying change already happened on disk — so this just records a
    // successful acknowledgment rather than calling into the plugin.
    for action in &loaded_actions {
        let output = serde_json::json!({ "kind": action.kind, "acknowledged": true });
        exec.record_action_result(ActionResult {
            action: action.id.clone(),
            status: ActionStatus::Succeeded,
            output: output.clone(),
        })
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        storage
            .insert_action_result(
                exec.id().as_str(),
                action.id.as_str(),
                "succeeded",
                &output.to_string(),
                &Timestamp::now().to_rfc3339(),
            )
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    }

    exec.begin_verification();

    // Section 16: verification is explicit, not "the command exited 0".
    // Re-observe every changed target and compare to the Change's
    // recorded `after` snapshot.
    let mut all_verified = true;
    for change_id in &change_ids {
        let change_row = storage
            .get_change(change_id)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?
            .ok_or_else(|| anyhow::anyhow!("missing change {change_id}"))?;
        let expected = to_btree(&serde_json::from_str(&change_row.after_json)?);
        let observations = fs_plugin
            .observe(&change_row.target)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let mut observed = BTreeMap::new();
        for obs in observations {
            observed.insert(obs.attribute, obs.value);
        }
        let target = ResourceId::parse(change_row.target.clone())?;
        let verification = Verification::check(target, expected.clone(), observed.clone());
        let evidence_id = forge_core::EvidenceId::new();
        storage
            .insert_evidence(
                evidence_id.as_str(),
                "verification",
                &Timestamp::now().to_rfc3339(),
                Some(&change_row.target),
                Some(change_id),
                Some(&plan_row.id),
                Some(exec.id().as_str()),
                None,
                Some(&serde_json::to_string(&expected)?),
                Some(&serde_json::to_string(&observed)?),
                Some(if verification.passed() { "passed" } else { "failed" }),
            )
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        if !verification.passed() {
            all_verified = false;
        }
    }

    exec.complete_with_verification(all_verified);

    if exec.state() == forge_state::ExecutionState::Committed {
        storage
            .update_execution_state(exec.id().as_str(), "COMMITTED", Some(&Timestamp::now().to_rfc3339()))
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        println!("EXECUTION COMMITTED");
        println!();
        println!("  plan:       {}", plan_row.id);
        println!("  execution:  {}", exec.id());
        println!("  actions:    {} succeeded", loaded_actions.len());
        return Ok(());
    }

    // Section 27: verification failed -> compensate.
    storage
        .update_execution_state(exec.id().as_str(), "FAILED", None)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let compensation_plan = forge_compensate::CompensationPlan::from_executed_actions(&loaded_actions);
    exec.begin_compensation().map_err(|e| anyhow::anyhow!(e.to_string()))?;
    storage
        .update_execution_state(exec.id().as_str(), "COMPENSATING", None)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    // v0.1: every action this CLI can currently produce
    // ("filesystem.acknowledge_change") declares Unsupported compensation
    // honestly (see plan.rs) — so compensation cannot succeed. This is
    // reported accurately (Section 17/30), never silently masked as a
    // rollback that didn't happen.
    let all_steps_succeeded = compensation_plan.fully_supported();
    exec.complete_compensation(all_steps_succeeded, compensation_plan.has_gaps());

    let final_state_str = match exec.state() {
        forge_state::ExecutionState::Compensated => "COMPENSATED",
        forge_state::ExecutionState::CompensationFailed => "COMPENSATION_FAILED",
        _ => "UNKNOWN",
    };
    storage
        .update_execution_state(exec.id().as_str(), final_state_str, Some(&Timestamp::now().to_rfc3339()))
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    println!("EXECUTION FAILED");
    println!();
    println!("Verification:");
    println!("  FAILED");
    println!();
    println!("Compensation:");
    if compensation_plan.has_gaps() {
        println!("  UNSUPPORTED for {} of {} action(s) — see `forge evidence` for details",
            compensation_plan.steps.iter().filter(|s| s.inverse_description.is_none()).count(),
            compensation_plan.steps.len());
    } else {
        println!("  {}", if all_steps_succeeded { "SUCCESS" } else { "FAILED" });
    }
    println!();
    println!("Final state:");
    println!("  {final_state_str}");
    Ok(())
}
