# ADR 0003: Change, Plan (post-approval), and Evidence Are Immutable

## Status
Accepted

## Context
Section 10 requires Change to be immutable after creation; Section 13
requires Plan to be immutable after approval; Section 18 requires Evidence
to be immutable. Rust cannot enforce database-level immutability by itself.

## Decision
- At the schema level, `changes`, `evidence`, and `observations` tables have
  no `UPDATE` statement anywhere in the codebase — only `INSERT`. This is a
  code-review-enforced invariant, checked by an integration test that greps
  the storage crate for forbidden `UPDATE changes`/`UPDATE evidence` SQL.
- `Plan` is mutable only while in `DRAFT` state (in-memory, not yet
  persisted as approved). Once persisted with `approved_at` set, all
  mutating methods on `Plan` return `Err(PlanError::Immutable)`.
- Revalidation (Section 14) never mutates a Plan's `changes`/`actions`; it
  either confirms validity or produces a new `PlanStatus::Stale` record and
  requires a brand-new Plan.

## Consequences
- Immutability is a combination of Rust type-state (for Plan) and a
  storage-layer discipline (for Change/Evidence/Observation), verified by
  tests rather than by the type system alone. This is documented so future
  contributors don't "optimize" by adding an UPDATE.
