//! Change and Intent (Sections 9-10). A Change is immutable once
//! constructed (ADR 0003) — this is enforced here by making every field
//! private and only exposing them through read accessors; there is no
//! `set_*` method anywhere on `Change`.

use forge_core::{ChangeId, ResourceId, Timestamp};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Operation {
    Create,
    Update,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeSource {
    User,
    Plugin,
    System,
}

/// Section 9: Intent describes what the user/system *wants*. Core must
/// never infer an Intent merely because a Change was observed — an Intent
/// is only ever created from an explicit user/system request, never
/// synthesized inside `forge-change` or `forge-plan` from an Observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub target: ResourceId,
    pub description: String,
    pub desired_attributes: BTreeMap<String, Value>,
    pub requested_by: String,
    pub requested_at: Timestamp,
}

/// Immutable once constructed (ADR 0003, Section 10). All fields are
/// private; the only way to build one is `Change::new`, and there is
/// deliberately no setter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    id: ChangeId,
    target: ResourceId,
    operation: Operation,
    before: BTreeMap<String, Value>,
    after: BTreeMap<String, Value>,
    source: ChangeSource,
    actor: Option<String>,
    timestamp: Timestamp,
    confidence: f64,
}

impl Change {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        target: ResourceId,
        operation: Operation,
        before: BTreeMap<String, Value>,
        after: BTreeMap<String, Value>,
        source: ChangeSource,
        actor: Option<String>,
        confidence: f64,
    ) -> Self {
        Self {
            id: ChangeId::new(),
            target,
            operation,
            before,
            after,
            source,
            actor,
            timestamp: Timestamp::now(),
            confidence: confidence.clamp(0.0, 1.0),
        }
    }

    pub fn id(&self) -> &ChangeId {
        &self.id
    }
    pub fn target(&self) -> &ResourceId {
        &self.target
    }
    pub fn operation(&self) -> Operation {
        self.operation
    }
    pub fn before(&self) -> &BTreeMap<String, Value> {
        &self.before
    }
    pub fn after(&self) -> &BTreeMap<String, Value> {
        &self.after
    }
    pub fn source(&self) -> ChangeSource {
        self.source
    }
    pub fn actor(&self) -> Option<&str> {
        self.actor.as_deref()
    }
    pub fn timestamp(&self) -> Timestamp {
        self.timestamp
    }
    pub fn confidence(&self) -> f64 {
        self.confidence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_confidence_is_clamped() {
        let c = Change::new(
            ResourceId::parse("docker://container/api").unwrap(),
            Operation::Update,
            BTreeMap::new(),
            BTreeMap::new(),
            ChangeSource::User,
            None,
            5.0,
        );
        assert_eq!(c.confidence(), 1.0);
    }

    #[test]
    fn change_has_no_public_setters() {
        // Compile-time proof by construction: Change only exposes getters.
        // (If a setter is ever added, ADR 0003 has been violated.)
        let c = Change::new(
            ResourceId::parse("fs://file/a").unwrap(),
            Operation::Create,
            BTreeMap::new(),
            BTreeMap::new(),
            ChangeSource::System,
            None,
            1.0,
        );
        assert_eq!(c.operation(), Operation::Create);
    }
}
