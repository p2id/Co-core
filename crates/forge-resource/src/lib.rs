//! Resource, Observation, and Relation — Sections 6, 8, 11 of the master
//! context. A Resource is anything observable or manageable; an
//! Observation is a fact a plugin discovered about one; a Relation links
//! two resources together.
//!
//! Observation != Intent != Desired State != Verification (Section 8) —
//! this crate only models the "fact discovered by a plugin" side. Intent
//! and desired state live in `forge-change`; verification lives in
//! `forge-verify`.

use forge_core::{ResourceId, Timestamp};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub id: ResourceId,
    pub attributes: BTreeMap<String, Value>,
    pub observed_at: Timestamp,
    /// Name of the plugin that produced this observation, e.g. "docker".
    pub source: String,
}

impl Resource {
    pub fn new(id: ResourceId, source: impl Into<String>) -> Self {
        Self {
            id,
            attributes: BTreeMap::new(),
            observed_at: Timestamp::now(),
            source: source.into(),
        }
    }

    pub fn provider(&self) -> &str {
        self.id.provider()
    }

    pub fn kind(&self) -> &str {
        self.id.kind()
    }
}

/// A single discovered fact about a resource (Section 8). Never mutated —
/// stored append-only (ADR 0002).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub resource: ResourceId,
    pub attribute: String,
    pub value: Value,
    pub observed_at: Timestamp,
    pub source: String,
}

/// Relation kinds are fixed in v0.1 (Section 11). Plugins may discover
/// relations but must not invent new kinds outside this set — Core does
/// not hardcode domain-specific semantics beyond this shared vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    DependsOn,
    Consumes,
    Produces,
    Exposes,
    Modifies,
    Contains,
    RunsOn,
    References,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub from: ResourceId,
    pub to: ResourceId,
    pub kind: RelationKind,
    pub discovered_by: String,
    pub discovered_at: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_exposes_provider_and_kind_from_id() {
        let id = ResourceId::parse("git://repository/project").unwrap();
        let r = Resource::new(id, "git");
        assert_eq!(r.provider(), "git");
        assert_eq!(r.kind(), "repository");
    }
}
