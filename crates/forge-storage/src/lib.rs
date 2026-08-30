//! SQLite persistence (ADR 0002). Current-state tables (`resources`,
//! `relations`) are mutated normally; append-only tables (`changes`,
//! `observations`, `plans`, `actions`, `executions`, `action_results`,
//! `evidence`) are only ever the target of `INSERT` in this module — see
//! ADR 0003 and `tests::no_update_on_append_only_tables` below, which
//! greps this file's own SQL literals to catch a future regression.

use forge_core::ForgeError;
use rusqlite::{Connection, OptionalExtension};

pub const MIGRATION_0001: &str = include_str!("../migrations/0001_init.sql");

/// Tables that must never appear as the target of an UPDATE statement
/// anywhere in this crate (ADR 0003), with two documented exceptions: see
/// `update_execution_state` and `mark_plan_stale`'s doc comments for why
/// `executions.state` and `plans.status` are allowed to transition in
/// place. Both are intentionally excluded from the list below rather than
/// silently violating the same guarantee this list checks for every other
/// table — the guard test only scans for the tables actually listed here.
pub const APPEND_ONLY_TABLES: &[&str] = &[
    "observations",
    "changes",
    "actions",
    "action_results",
    "evidence",
];

pub struct Storage {
    conn: Connection,
}

impl Storage {
    pub fn open(path: &str) -> Result<Self, ForgeError> {
        let conn = Connection::open(path).map_err(|e| ForgeError::Storage(e.to_string()))?;
        let storage = Self { conn };
        storage.migrate()?;
        Ok(storage)
    }

    pub fn open_in_memory() -> Result<Self, ForgeError> {
        let conn = Connection::open_in_memory().map_err(|e| ForgeError::Storage(e.to_string()))?;
        let storage = Self { conn };
        storage.migrate()?;
        Ok(storage)
    }

    fn migrate(&self) -> Result<(), ForgeError> {
        self.conn
            .execute_batch(MIGRATION_0001)
            .map_err(|e| ForgeError::Storage(e.to_string()))
    }

    pub fn upsert_resource(
        &self,
        id: &str,
        provider: &str,
        kind: &str,
        attributes_json: &str,
        observed_at: &str,
        source: &str,
    ) -> Result<(), ForgeError> {
        // `resources` is a mutable current-state table (ADR 0002) — this
        // INSERT ... ON CONFLICT is intentional and NOT a violation of the
        // append-only rule, which applies only to APPEND_ONLY_TABLES.
        self.conn
            .execute(
                "INSERT INTO resources (id, provider, kind, attributes, observed_at, source)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET
                    attributes = excluded.attributes,
                    observed_at = excluded.observed_at,
                    source = excluded.source",
                rusqlite::params![id, provider, kind, attributes_json, observed_at, source],
            )
            .map_err(|e| ForgeError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn insert_change(
        &self,
        id: &str,
        target: &str,
        operation: &str,
        before_json: &str,
        after_json: &str,
        source: &str,
        actor: Option<&str>,
        timestamp: &str,
        confidence: f64,
    ) -> Result<(), ForgeError> {
        self.conn
            .execute(
                "INSERT INTO changes (id, target, operation, before, after, source, actor, timestamp, confidence)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    id, target, operation, before_json, after_json, source, actor, timestamp,
                    confidence
                ],
            )
            .map_err(|e| ForgeError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn insert_evidence(
        &self,
        id: &str,
        evidence_type: &str,
        timestamp: &str,
        resource: Option<&str>,
        change_id: Option<&str>,
        plan_id: Option<&str>,
        execution_id: Option<&str>,
        action_id: Option<&str>,
        expected_json: Option<&str>,
        observed_json: Option<&str>,
        result: Option<&str>,
    ) -> Result<(), ForgeError> {
        self.conn
            .execute(
                "INSERT INTO evidence
                    (id, type, timestamp, resource, change_id, plan_id, execution_id, action_id, expected, observed, result)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                rusqlite::params![
                    id, evidence_type, timestamp, resource, change_id, plan_id, execution_id,
                    action_id, expected_json, observed_json, result
                ],
            )
            .map_err(|e| ForgeError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn evidence_for_change(&self, change_id: &str) -> Result<Vec<String>, ForgeError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM evidence WHERE change_id = ?1 ORDER BY timestamp ASC")
            .map_err(|e| ForgeError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map([change_id], |row| row.get::<_, String>(0))
            .map_err(|e| ForgeError::Storage(e.to_string()))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| ForgeError::Storage(e.to_string()))
    }

    pub fn count_resources(&self) -> Result<i64, ForgeError> {
        self.conn
            .query_row("SELECT COUNT(*) FROM resources", [], |r| r.get(0))
            .map_err(|e| ForgeError::Storage(e.to_string()))
    }

    // ===================== Readers (Phase 4 follow-up) =====================
    // These were the missing pieces called out in README.md's "Continuing
    // the work" step 2: the tables existed since ADR 0002's migration but
    // had no reader. Added here so `forge changes`, `forge inspect`, and
    // `forge history` can be wired to real data instead of stub output.

    pub fn get_resource(&self, id: &str) -> Result<Option<ResourceRow>, ForgeError> {
        self.conn
            .query_row(
                "SELECT id, provider, kind, attributes, observed_at, source
                 FROM resources WHERE id = ?1",
                [id],
                |r| {
                    Ok(ResourceRow {
                        id: r.get(0)?,
                        provider: r.get(1)?,
                        kind: r.get(2)?,
                        attributes_json: r.get(3)?,
                        observed_at: r.get(4)?,
                        source: r.get(5)?,
                    })
                },
            )
            .optional()
            .map_err(|e| ForgeError::Storage(e.to_string()))
    }

    pub fn list_resources(&self) -> Result<Vec<ResourceRow>, ForgeError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, provider, kind, attributes, observed_at, source
                 FROM resources ORDER BY id ASC",
            )
            .map_err(|e| ForgeError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map([], |r| {
                Ok(ResourceRow {
                    id: r.get(0)?,
                    provider: r.get(1)?,
                    kind: r.get(2)?,
                    attributes_json: r.get(3)?,
                    observed_at: r.get(4)?,
                    source: r.get(5)?,
                })
            })
            .map_err(|e| ForgeError::Storage(e.to_string()))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| ForgeError::Storage(e.to_string()))
    }

    pub fn list_changes(&self, limit: i64) -> Result<Vec<ChangeRow>, ForgeError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, target, operation, before, after, source, actor, timestamp, confidence
                 FROM changes ORDER BY timestamp DESC LIMIT ?1",
            )
            .map_err(|e| ForgeError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map([limit], |r| {
                Ok(ChangeRow {
                    id: r.get(0)?,
                    target: r.get(1)?,
                    operation: r.get(2)?,
                    before_json: r.get(3)?,
                    after_json: r.get(4)?,
                    source: r.get(5)?,
                    actor: r.get(6)?,
                    timestamp: r.get(7)?,
                    confidence: r.get(8)?,
                })
            })
            .map_err(|e| ForgeError::Storage(e.to_string()))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| ForgeError::Storage(e.to_string()))
    }

    pub fn list_evidence(&self, limit: i64) -> Result<Vec<EvidenceRow>, ForgeError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, type, timestamp, resource, change_id, plan_id, execution_id, action_id, expected, observed, result
                 FROM evidence ORDER BY timestamp DESC LIMIT ?1",
            )
            .map_err(|e| ForgeError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map([limit], |r| {
                Ok(EvidenceRow {
                    id: r.get(0)?,
                    evidence_type: r.get(1)?,
                    timestamp: r.get(2)?,
                    resource: r.get(3)?,
                    change_id: r.get(4)?,
                    plan_id: r.get(5)?,
                    execution_id: r.get(6)?,
                    action_id: r.get(7)?,
                    expected_json: r.get(8)?,
                    observed_json: r.get(9)?,
                    result: r.get(10)?,
                })
            })
            .map_err(|e| ForgeError::Storage(e.to_string()))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| ForgeError::Storage(e.to_string()))
    }

    // ===================== Plan / Action / Execution writers =====================
    // Needed to persist what forge-plan / forge-exec produce in-memory
    // (Phase 5/7 wiring). Plans and actions are append-only per ADR 0003;
    // `update_execution_state` is the one documented, audited exception
    // for the `executions.state` column (see module doc + ADR 0003).

    #[allow(clippy::too_many_arguments)]
    pub fn insert_plan(
        &self,
        id: &str,
        status: &str,
        changes_json: &str,
        actions_json: &str,
        ordering_json: &str,
        constraints_json: &str,
        risk: &str,
        verification_strategy: Option<&str>,
        compensation_strategy: Option<&str>,
        created_at: &str,
        approved_at: Option<&str>,
    ) -> Result<(), ForgeError> {
        self.conn
            .execute(
                "INSERT INTO plans (id, status, changes, actions, ordering, constraints, risk,
                                     verification_strategy, compensation_strategy, created_at, approved_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                rusqlite::params![
                    id, status, changes_json, actions_json, ordering_json, constraints_json,
                    risk, verification_strategy, compensation_strategy, created_at, approved_at
                ],
            )
            .map_err(|e| ForgeError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn insert_action(
        &self,
        id: &str,
        change_id: &str,
        kind: &str,
        preconditions_json: &str,
        compensation: &str,
    ) -> Result<(), ForgeError> {
        self.conn
            .execute(
                "INSERT INTO actions (id, change_id, kind, preconditions, compensation)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![id, change_id, kind, preconditions_json, compensation],
            )
            .map_err(|e| ForgeError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn insert_execution(
        &self,
        id: &str,
        plan_id: &str,
        state: &str,
        started_at: &str,
    ) -> Result<(), ForgeError> {
        self.conn
            .execute(
                "INSERT INTO executions (id, plan_id, state, started_at, finished_at)
                 VALUES (?1, ?2, ?3, ?4, NULL)",
                rusqlite::params![id, plan_id, state, started_at],
            )
            .map_err(|e| ForgeError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Deliberate, sole exception to "append-only tables are INSERT-only":
    /// an Execution's `state` legitimately transitions in place
    /// (EXECUTING -> VERIFYING -> COMMITTED, etc, per Section 15) and the
    /// row identity (which plan, when started) never changes — only the
    /// state and finish time do. This is why `executions` carries a
    /// comment in `APPEND_ONLY_TABLES` rather than being silently treated
    /// like `changes`/`evidence`. Every transition is still independently
    /// visible via `action_results` and `evidence`, so nothing about the
    /// history is lost even though this one column is mutable.
    pub fn update_execution_state(
        &self,
        execution_id: &str,
        state: &str,
        finished_at: Option<&str>,
    ) -> Result<(), ForgeError> {
        self.conn
            .execute(
                "UPDATE executions SET state = ?2, finished_at = ?3 WHERE id = ?1",
                rusqlite::params![execution_id, state, finished_at],
            )
            .map_err(|e| ForgeError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn insert_action_result(
        &self,
        execution_id: &str,
        action_id: &str,
        status: &str,
        output_json: &str,
        recorded_at: &str,
    ) -> Result<(), ForgeError> {
        self.conn
            .execute(
                "INSERT INTO action_results (execution_id, action_id, status, output, recorded_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![execution_id, action_id, status, output_json, recorded_at],
            )
            .map_err(|e| ForgeError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn get_plan(&self, id: &str) -> Result<Option<PlanRow>, ForgeError> {
        self.conn
            .query_row(
                "SELECT id, status, changes, actions, ordering, constraints, risk,
                        verification_strategy, compensation_strategy, created_at, approved_at
                 FROM plans WHERE id = ?1",
                [id],
                |r| {
                    Ok(PlanRow {
                        id: r.get(0)?,
                        status: r.get(1)?,
                        changes_json: r.get(2)?,
                        actions_json: r.get(3)?,
                        ordering_json: r.get(4)?,
                        constraints_json: r.get(5)?,
                        risk: r.get(6)?,
                        verification_strategy: r.get(7)?,
                        compensation_strategy: r.get(8)?,
                        created_at: r.get(9)?,
                        approved_at: r.get(10)?,
                    })
                },
            )
            .optional()
            .map_err(|e| ForgeError::Storage(e.to_string()))
    }

    pub fn get_action(&self, id: &str) -> Result<Option<ActionRow>, ForgeError> {
        self.conn
            .query_row(
                "SELECT id, change_id, kind, preconditions, compensation FROM actions WHERE id = ?1",
                [id],
                |r| {
                    Ok(ActionRow {
                        id: r.get(0)?,
                        change_id: r.get(1)?,
                        kind: r.get(2)?,
                        preconditions_json: r.get(3)?,
                        compensation: r.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(|e| ForgeError::Storage(e.to_string()))
    }

    pub fn get_change(&self, id: &str) -> Result<Option<ChangeRow>, ForgeError> {
        self.conn
            .query_row(
                "SELECT id, target, operation, before, after, source, actor, timestamp, confidence
                 FROM changes WHERE id = ?1",
                [id],
                |r| {
                    Ok(ChangeRow {
                        id: r.get(0)?,
                        target: r.get(1)?,
                        operation: r.get(2)?,
                        before_json: r.get(3)?,
                        after_json: r.get(4)?,
                        source: r.get(5)?,
                        actor: r.get(6)?,
                        timestamp: r.get(7)?,
                        confidence: r.get(8)?,
                    })
                },
            )
            .optional()
            .map_err(|e| ForgeError::Storage(e.to_string()))
    }

    /// Most recently created plan whose status is still `APPROVED` (i.e.
    /// not yet executed/stale) — what `forge apply` with no explicit plan
    /// id would act on.
    pub fn latest_approved_plan(&self) -> Result<Option<PlanRow>, ForgeError> {
        self.conn
            .query_row(
                "SELECT id, status, changes, actions, ordering, constraints, risk,
                        verification_strategy, compensation_strategy, created_at, approved_at
                 FROM plans WHERE status = 'APPROVED' ORDER BY created_at DESC LIMIT 1",
                [],
                |r| {
                    Ok(PlanRow {
                        id: r.get(0)?,
                        status: r.get(1)?,
                        changes_json: r.get(2)?,
                        actions_json: r.get(3)?,
                        ordering_json: r.get(4)?,
                        constraints_json: r.get(5)?,
                        risk: r.get(6)?,
                        verification_strategy: r.get(7)?,
                        compensation_strategy: r.get(8)?,
                        created_at: r.get(9)?,
                        approved_at: r.get(10)?,
                    })
                },
            )
            .optional()
            .map_err(|e| ForgeError::Storage(e.to_string()))
    }

    /// Section 14/28: mark a previously-approved plan STALE. This is the
    /// documented, narrow exception (alongside `update_execution_state`)
    /// to "plans is append-only" (ADR 0003) — the Plan's *content*
    /// (changes/actions/ordering/risk) is still never rewritten, only the
    /// `status` transitions to reflect that the world moved on since
    /// approval. Every other field is left untouched.
    pub fn mark_plan_stale(&self, plan_id: &str) -> Result<(), ForgeError> {
        self.conn
            .execute("UPDATE plans SET status = 'STALE' WHERE id = ?1", [plan_id])
            .map_err(|e| ForgeError::Storage(e.to_string()))?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ResourceRow {
    pub id: String,
    pub provider: String,
    pub kind: String,
    pub attributes_json: String,
    pub observed_at: String,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct ChangeRow {
    pub id: String,
    pub target: String,
    pub operation: String,
    pub before_json: String,
    pub after_json: String,
    pub source: String,
    pub actor: Option<String>,
    pub timestamp: String,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub struct EvidenceRow {
    pub id: String,
    pub evidence_type: String,
    pub timestamp: String,
    pub resource: Option<String>,
    pub change_id: Option<String>,
    pub plan_id: Option<String>,
    pub execution_id: Option<String>,
    pub action_id: Option<String>,
    pub expected_json: Option<String>,
    pub observed_json: Option<String>,
    pub result: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PlanRow {
    pub id: String,
    pub status: String,
    pub changes_json: String,
    pub actions_json: String,
    pub ordering_json: String,
    pub constraints_json: String,
    pub risk: String,
    pub verification_strategy: Option<String>,
    pub compensation_strategy: Option<String>,
    pub created_at: String,
    pub approved_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ActionRow {
    pub id: String,
    pub change_id: String,
    pub kind: String,
    pub preconditions_json: String,
    pub compensation: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_creates_all_expected_tables() {
        let storage = Storage::open_in_memory().unwrap();
        let mut stmt = storage
            .conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap();
        let names: Vec<String> = stmt
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        for expected in [
            "resources", "relations", "observations", "changes", "plans",
            "actions", "executions", "action_results", "evidence",
        ] {
            assert!(names.iter().any(|n| n == expected), "missing table {expected}");
        }
    }

    #[test]
    fn resource_upsert_then_count() {
        let storage = Storage::open_in_memory().unwrap();
        storage
            .upsert_resource(
                "docker://container/api",
                "docker",
                "container",
                "{}",
                "2026-01-01T00:00:00Z",
                "docker",
            )
            .unwrap();
        assert_eq!(storage.count_resources().unwrap(), 1);

        // upsert same id again should not create a duplicate row
        storage
            .upsert_resource(
                "docker://container/api",
                "docker",
                "container",
                "{\"image\":\"api:v1.5\"}",
                "2026-01-02T00:00:00Z",
                "docker",
            )
            .unwrap();
        assert_eq!(storage.count_resources().unwrap(), 1);
    }

    #[test]
    fn evidence_is_queryable_by_change() {
        let storage = Storage::open_in_memory().unwrap();
        storage
            .insert_change(
                "chg_test1", "docker://container/api", "update", "{}", "{}", "user", None,
                "2026-01-01T00:00:00Z", 1.0,
            )
            .unwrap();
        storage
            .insert_evidence(
                "ev_test1", "verification", "2026-01-01T00:00:01Z", None,
                Some("chg_test1"), None, None, None, None, None, Some("passed"),
            )
            .unwrap();
        let found = storage.evidence_for_change("chg_test1").unwrap();
        assert_eq!(found, vec!["ev_test1".to_string()]);
    }

    /// ADR 0003 enforcement: this crate's own source must never contain an
    /// UPDATE statement targeting an append-only table. This is a coarse
    /// but effective guard against a future regression.
    #[test]
    fn no_update_on_append_only_tables() {
        let source = include_str!("lib.rs");
        for table in APPEND_ONLY_TABLES {
            let forbidden = format!("UPDATE {table}");
            assert!(
                !source.to_uppercase().contains(&forbidden.to_uppercase()),
                "found forbidden UPDATE on append-only table: {table}"
            );
        }
    }
}

#[cfg(test)]
mod extra_tests {
    use super::*;

    #[test]
    fn plan_and_action_roundtrip() {
        let storage = Storage::open_in_memory().unwrap();
        storage
            .insert_change(
                "chg_x", "fs://file/a.txt", "update", "{}", "{}", "user", None,
                "2026-01-01T00:00:00Z", 1.0,
            )
            .unwrap();
        storage
            .insert_action("act_x", "chg_x", "filesystem.write", "[]", "partial")
            .unwrap();
        storage
            .insert_plan(
                "plan_x", "APPROVED", "[\"chg_x\"]", "[\"act_x\"]", "[\"act_x\"]", "[]",
                "LOW", None, None, "2026-01-01T00:00:00Z", Some("2026-01-01T00:00:01Z"),
            )
            .unwrap();

        let plan = storage.get_plan("plan_x").unwrap().expect("plan exists");
        assert_eq!(plan.status, "APPROVED");
        let action = storage.get_action("act_x").unwrap().expect("action exists");
        assert_eq!(action.change_id, "chg_x");

        let latest = storage.latest_approved_plan().unwrap().expect("one approved plan");
        assert_eq!(latest.id, "plan_x");
    }

    #[test]
    fn execution_state_transition_is_visible_via_get() {
        let storage = Storage::open_in_memory().unwrap();
        storage
            .insert_plan(
                "plan_y", "APPROVED", "[]", "[]", "[]", "[]", "LOW", None, None,
                "2026-01-01T00:00:00Z", Some("2026-01-01T00:00:00Z"),
            )
            .unwrap();
        storage
            .insert_execution("exec_y", "plan_y", "EXECUTING", "2026-01-01T00:00:05Z")
            .unwrap();
        storage
            .update_execution_state("exec_y", "COMMITTED", Some("2026-01-01T00:00:10Z"))
            .unwrap();

        let state: String = storage
            .conn
            .query_row("SELECT state FROM executions WHERE id = 'exec_y'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(state, "COMMITTED");
    }

    #[test]
    fn list_changes_orders_newest_first() {
        let storage = Storage::open_in_memory().unwrap();
        storage
            .insert_change("chg_1", "fs://file/a", "update", "{}", "{}", "user", None, "2026-01-01T00:00:00Z", 1.0)
            .unwrap();
        storage
            .insert_change("chg_2", "fs://file/a", "update", "{}", "{}", "user", None, "2026-01-02T00:00:00Z", 1.0)
            .unwrap();
        let changes = storage.list_changes(10).unwrap();
        assert_eq!(changes[0].id, "chg_2");
        assert_eq!(changes[1].id, "chg_1");
    }
}
