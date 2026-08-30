//! `forge plan` (Section 24, Phase 5). v0.1 scope: diffs a fresh
//! filesystem scan against the resources stored by the last `forge scan`,
//! builds a Change + Action per drifted resource, assesses risk
//! (ADR 0006), and persists an APPROVED Plan — refusing to auto-approve
//! when policy or Section 30 (irreversible + CRITICAL) requires explicit
//! operator approval, since this CLI has no interactive prompt yet.
//!
//! Git/Docker plugins aren't implemented (Phases 13-14), so `forge-graph`
//! blast-radius/dependency wiring has nothing to traverse yet for a
//! filesystem-only demo; `dependency_count` is left at 0 here rather than
//! faked, and is the first thing to wire once a plugin reports relations.

use forge_change::{Change, ChangeSource, Operation};
use forge_core::{ChangeId, CompensationSupport, ResourceId, Timestamp};
use forge_plan::{assess_risk, Action, ActionId, Environment, Plan, RiskFactors};
use forge_plugin::Plugin;
use forge_plugin_filesystem::FilesystemPlugin;
use forge_policy::{Decision, Policy};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

fn to_btree(v: &Value) -> BTreeMap<String, Value> {
    v.as_object()
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default()
}

fn compensation_str(c: CompensationSupport) -> &'static str {
    match c {
        CompensationSupport::Supported => "supported",
        CompensationSupport::Partial => "partial",
        CompensationSupport::Unsupported => "unsupported",
    }
}

fn operation_str(op: Operation) -> &'static str {
    match op {
        Operation::Create => "create",
        Operation::Update => "update",
        Operation::Delete => "delete",
    }
}

fn source_str(s: ChangeSource) -> &'static str {
    match s {
        ChangeSource::User => "user",
        ChangeSource::Plugin => "plugin",
        ChangeSource::System => "system",
    }
}

pub fn run(yes: bool) -> anyhow::Result<()> {
    let storage = super::open_storage()?;

    // Last known state (as of the most recent `forge scan`).
    let stored = storage.list_resources().map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let mut stored_map: BTreeMap<String, String> = BTreeMap::new();
    for r in &stored {
        stored_map.insert(r.id.clone(), r.attributes_json.clone());
    }

    // Fresh observation — NOT persisted here; that stays `forge scan`'s
    // job (Section 24 keeps `scan` and `plan` as separate commands).
    let plugin = FilesystemPlugin::new(".");
    let fresh = plugin.scan().map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let mut fresh_map: BTreeMap<String, Value> = BTreeMap::new();
    for r in &fresh {
        if r.id.as_str().starts_with("fs://file/.forge/") {
            continue;
        }
        fresh_map.insert(r.id.as_str().to_string(), serde_json::to_value(&r.attributes)?);
    }

    let fresh_ids: BTreeSet<String> = fresh_map.keys().cloned().collect();
    let stored_ids: BTreeSet<String> = stored_map.keys().cloned().collect();

    let mut changes: Vec<Change> = Vec::new();
    let mut actions: Vec<Action> = Vec::new();
    let mut ordering: Vec<ActionId> = Vec::new();

    for id in fresh_ids.union(&stored_ids) {
        let target = ResourceId::parse(id.clone())?;
        let after_val = fresh_map.get(id);
        let before_val: Option<Value> = stored_map
            .get(id)
            .map(|s| serde_json::from_str(s).unwrap_or(Value::Null));

        let (op, before, after) = match (&before_val, after_val) {
            (None, Some(a)) => (Operation::Create, BTreeMap::new(), to_btree(a)),
            (Some(b), None) => (Operation::Delete, to_btree(b), BTreeMap::new()),
            (Some(b), Some(a)) if b != a => (Operation::Update, to_btree(b), to_btree(a)),
            _ => continue, // unchanged — not a Change (Section 9/10: only real drift)
        };

        let change = Change::new(target.clone(), op, before, after, ChangeSource::System, None, 1.0);
        let action = Action {
            id: ActionId::new(),
            change: change.id().clone(),
            kind: "filesystem.acknowledge_change".to_string(),
            preconditions: vec![format!(
                "{target} still matches the detected after-state at apply time"
            )],
            // Honest per ADR 0004/Section 17: we did not capture enough
            // information (a full content snapshot) to safely reconstruct
            // the previous file, so this is Unsupported, not assumed.
            compensation: CompensationSupport::Unsupported,
        };
        ordering.push(action.id.clone());
        actions.push(action);
        changes.push(change);
    }

    if changes.is_empty() {
        println!("No drift detected since last `forge scan` — nothing to plan.");
        return Ok(());
    }

    let risk = assess_risk(RiskFactors {
        blast_radius: changes.len(),
        environment: Environment::Development,
        privileged: false,
        compensation: CompensationSupport::Unsupported,
        dependency_count: 0, // TODO: wire forge-graph once a plugin reports Relations
    });

    // Section 23/30: default-deny policy. This v0.1 CLI has no
    // interactive approval prompt, so `--yes` is the operator's explicit
    // approval signal — refusing to silently upgrade DENY to ALLOW.
    let policy = Policy::default();
    let decision = policy.execution_decision(risk, CompensationSupport::Unsupported);

    if decision == Decision::RequireApproval && !yes {
        println!("PLAN (not persisted)");
        println!();
        println!("  changes: {}", changes.len());
        println!("  risk:    {risk}");
        println!();
        println!("Policy requires explicit approval before this plan can be committed.");
        println!("Re-run with `forge plan --yes` to approve and persist it.");
        return Ok(());
    }

    let change_ids: Vec<ChangeId> = changes.iter().map(|c| c.id().clone()).collect();
    let plan = Plan::new_draft(change_ids.clone(), actions.clone(), ordering.clone(), risk).approve();

    for change in &changes {
        storage
            .insert_change(
                change.id().as_str(),
                change.target().as_str(),
                operation_str(change.operation()),
                &serde_json::to_string(change.before())?,
                &serde_json::to_string(change.after())?,
                source_str(change.source()),
                change.actor(),
                &change.timestamp().to_rfc3339(),
                change.confidence(),
            )
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    }

    for action in &actions {
        storage
            .insert_action(
                action.id.as_str(),
                action.change.as_str(),
                &action.kind,
                &serde_json::to_string(&action.preconditions)?,
                compensation_str(action.compensation),
            )
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    }

    let now = Timestamp::now().to_rfc3339();
    let changes_json = serde_json::to_string(
        &change_ids.iter().map(|c| c.as_str().to_string()).collect::<Vec<_>>(),
    )?;
    let actions_json = serde_json::to_string(
        &actions.iter().map(|a| a.id.as_str().to_string()).collect::<Vec<_>>(),
    )?;
    let ordering_json = serde_json::to_string(
        &ordering.iter().map(|a| a.as_str().to_string()).collect::<Vec<_>>(),
    )?;

    storage
        .insert_plan(
            plan.id().as_str(),
            "APPROVED",
            &changes_json,
            &actions_json,
            &ordering_json,
            "[]",
            &risk.to_string(),
            None,
            None,
            &now,
            Some(&now),
        )
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    println!("PLAN {}", plan.id());
    println!();
    println!("  changes: {}", changes.len());
    println!("  risk:    {risk}");
    println!();
    println!("Run `forge apply` to execute, revalidate, verify, and commit.");
    Ok(())
}
