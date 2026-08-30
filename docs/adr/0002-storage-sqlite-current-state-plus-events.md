# ADR 0002: SQLite for Current State, Append-Only Events for History

## Status
Accepted

## Context
Full event sourcing (rebuilding all state by replaying every event) gives
strong auditability but adds significant complexity (snapshotting, replay
performance, migration of historical event shapes) that v0.1 does not need.
Pure "just overwrite the row" current-state storage loses the audit trail
Evidence (Section 18) requires.

## Decision
Use a hybrid model in SQLite:
- Mutable "current state" tables (`resources`, `relations`) queried directly
  for `forge state`, `forge inspect`.
- Append-only tables (`changes`, `observations`, `plans`, `executions`,
  `evidence`) that are never updated or deleted after insert, only inserted.
- Foreign keys tie append-only rows back to the resource they concern, so
  `forge evidence <change>` can query efficiently without replay.

## Consequences
- Current-state reads stay O(1)-ish indexed lookups, not O(n) replay.
- History/auditability is preserved via the append-only tables.
- We explicitly do NOT implement snapshot/replay-based event sourcing in
  v0.1 (Section 19 of master context).
