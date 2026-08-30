//! Dependency graph over Relations (Section 11), used by `forge-plan` to
//! answer "what will be affected?" (Section: fundamental problem, item 5)
//! before constructing a Plan.

use forge_core::ResourceId;
use forge_resource::Relation;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Default)]
pub struct DependencyGraph {
    edges: BTreeMap<ResourceId, Vec<Relation>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_relation(&mut self, relation: Relation) {
        self.edges
            .entry(relation.from.clone())
            .or_default()
            .push(relation);
    }

    /// Resources reachable from `start` by following outgoing relations,
    /// i.e. everything that might be affected if `start` changes.
    /// This directly answers fundamental-problem question 5: "what will
    /// be affected?"
    pub fn affected_by(&self, start: &ResourceId) -> BTreeSet<ResourceId> {
        let mut seen = BTreeSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(start.clone());

        while let Some(current) = queue.pop_front() {
            if let Some(relations) = self.edges.get(&current) {
                for rel in relations {
                    if seen.insert(rel.to.clone()) {
                        queue.push_back(rel.to.clone());
                    }
                }
            }
        }
        seen
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core::Timestamp;
    use forge_resource::RelationKind;

    fn rel(from: &str, to: &str, kind: RelationKind) -> Relation {
        Relation {
            from: ResourceId::parse(from).unwrap(),
            to: ResourceId::parse(to).unwrap(),
            kind,
            discovered_by: "test".into(),
            discovered_at: Timestamp::now(),
        }
    }

    #[test]
    fn affected_by_follows_transitive_deps() {
        let mut g = DependencyGraph::new();
        g.add_relation(rel(
            "docker://container/api",
            "docker://container/worker",
            RelationKind::DependsOn,
        ));
        g.add_relation(rel(
            "docker://container/worker",
            "docker://database/postgres",
            RelationKind::DependsOn,
        ));

        let affected = g.affected_by(&ResourceId::parse("docker://container/api").unwrap());
        assert!(affected.contains(&ResourceId::parse("docker://container/worker").unwrap()));
        assert!(affected.contains(&ResourceId::parse("docker://database/postgres").unwrap()));
    }
}
