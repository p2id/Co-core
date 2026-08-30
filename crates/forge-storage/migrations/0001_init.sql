-- Forge storage schema (ADR 0002: mutable current-state tables +
-- append-only history tables). No table in the "append-only" group below
-- may ever be the target of an UPDATE statement anywhere in the codebase
-- (enforced by tests/no_update_on_immutable_tables.rs).

PRAGMA foreign_keys = ON;

-- ===================== Mutable current-state tables =====================

CREATE TABLE resources (
    id            TEXT PRIMARY KEY,       -- provider://kind/id
    provider      TEXT NOT NULL,
    kind          TEXT NOT NULL,
    attributes    TEXT NOT NULL,          -- JSON
    observed_at   TEXT NOT NULL,          -- RFC3339
    source        TEXT NOT NULL
);

CREATE TABLE relations (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    from_resource TEXT NOT NULL REFERENCES resources(id),
    to_resource   TEXT NOT NULL REFERENCES resources(id),
    kind          TEXT NOT NULL,          -- depends_on | consumes | produces | exposes | modifies | contains | runs_on | references
    discovered_by TEXT NOT NULL,          -- plugin name
    discovered_at TEXT NOT NULL,
    UNIQUE(from_resource, to_resource, kind)
);

-- ===================== Append-only history tables =====================

CREATE TABLE observations (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    resource      TEXT NOT NULL REFERENCES resources(id),
    attribute     TEXT NOT NULL,
    value         TEXT NOT NULL,          -- JSON-encoded scalar/object
    observed_at   TEXT NOT NULL,
    source        TEXT NOT NULL
);

CREATE TABLE changes (
    id            TEXT PRIMARY KEY,       -- chg_...
    target        TEXT NOT NULL REFERENCES resources(id),
    operation     TEXT NOT NULL CHECK (operation IN ('create','update','delete')),
    before        TEXT NOT NULL,          -- JSON
    after         TEXT NOT NULL,          -- JSON
    source        TEXT NOT NULL CHECK (source IN ('user','plugin','system')),
    actor         TEXT,
    timestamp     TEXT NOT NULL,
    confidence    REAL NOT NULL CHECK (confidence >= 0.0 AND confidence <= 1.0)
);

CREATE TABLE plans (
    id                     TEXT PRIMARY KEY,     -- plan_...
    status                 TEXT NOT NULL,
    changes                TEXT NOT NULL,        -- JSON array of change ids
    actions                TEXT NOT NULL,        -- JSON array of action ids
    ordering               TEXT NOT NULL,        -- JSON array of action ids in order
    constraints            TEXT NOT NULL DEFAULT '[]',
    risk                   TEXT NOT NULL CHECK (risk IN ('LOW','MEDIUM','HIGH','CRITICAL')),
    verification_strategy  TEXT,
    compensation_strategy  TEXT,
    created_at             TEXT NOT NULL,
    approved_at            TEXT              -- NULL until approved; once set, row is logically frozen
);

CREATE TABLE actions (
    id             TEXT PRIMARY KEY,   -- act_...
    change_id      TEXT NOT NULL REFERENCES changes(id),
    kind           TEXT NOT NULL,
    preconditions  TEXT NOT NULL DEFAULT '[]',  -- JSON array
    compensation   TEXT NOT NULL CHECK (compensation IN ('supported','partial','unsupported'))
);

CREATE TABLE executions (
    id           TEXT PRIMARY KEY,   -- exec_...
    plan_id      TEXT NOT NULL REFERENCES plans(id),
    state        TEXT NOT NULL,
    started_at   TEXT NOT NULL,
    finished_at  TEXT
);

CREATE TABLE action_results (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    execution_id  TEXT NOT NULL REFERENCES executions(id),
    action_id     TEXT NOT NULL REFERENCES actions(id),
    status        TEXT NOT NULL CHECK (status IN ('succeeded','failed','skipped')),
    output        TEXT NOT NULL DEFAULT '{}',
    recorded_at   TEXT NOT NULL
);

CREATE TABLE evidence (
    id            TEXT PRIMARY KEY,   -- ev_...
    type          TEXT NOT NULL,
    timestamp     TEXT NOT NULL,
    resource      TEXT REFERENCES resources(id),
    change_id     TEXT REFERENCES changes(id),
    plan_id       TEXT REFERENCES plans(id),
    execution_id  TEXT REFERENCES executions(id),
    action_id     TEXT REFERENCES actions(id),
    expected      TEXT,   -- JSON
    observed      TEXT,   -- JSON
    result        TEXT CHECK (result IN ('passed','failed'))
);

CREATE INDEX idx_changes_target ON changes(target);
CREATE INDEX idx_evidence_change ON evidence(change_id);
CREATE INDEX idx_evidence_plan ON evidence(plan_id);
CREATE INDEX idx_evidence_execution ON evidence(execution_id);
CREATE INDEX idx_relations_from ON relations(from_resource);
CREATE INDEX idx_relations_to ON relations(to_resource);
CREATE INDEX idx_action_results_execution ON action_results(execution_id);
