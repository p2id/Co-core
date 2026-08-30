# Contributing to Forge

Forge's value is the protocol/Core, not any single CLI or plugin. Every
contribution should be evaluated against one question, taken directly
from the project's founding principles:

> Does this strengthen the Change Integrity Core?

If the honest answer is no, it probably does not belong in this
repository yet, however useful it may be in isolation.

## Before writing code

1. Read `docs/adr/` in order. Each ADR records a decision that was made
   deliberately, with tradeoffs considered — not an accident of how the
   first version happened to be written. If your change conflicts with an
   existing ADR, open a new ADR proposing to supersede it; do not just
   contradict it in code.
2. Read `RFC/0001-protocol-overview.md`. If your change affects a
   protocol primitive (Resource, Change, Plan, Execution, Evidence, ...),
   it likely needs a schema update in `schemas/` and, for anything
   non-trivial, a new or amended RFC.
3. Check `README.md`'s "What's stubbed or not started" section. It is
   kept current on purpose — start there before assuming something is
   unimplemented, and update it as part of your change once something
   moves from stubbed to real.

## Architectural invariants (non-negotiable without a new ADR)

These are enforced by convention and tests today, not by the compiler
alone — review changes with them in mind:

- Core domain crates (`crates/forge-*`, excluding `forge-cli`) never
  depend on `forge-cli` or any AI/LLM client (ADR 0001).
- `Change` and `Evidence` records are immutable after creation — no
  `UPDATE` statement may target their storage tables (ADR 0003; enforced
  by `forge-storage`'s `no_update_on_append_only_tables` test — read its
  doc comment before adding a new mutable column anywhere near these
  tables).
- A `Plan` is immutable after approval (ADR 0003; enforced by
  `forge-plan`'s type-state — mutating methods on an approved `Plan`
  return `Err`).
- A stale `Plan` can never execute (Section 14 of the original project
  brief; enforced by `Plan::ensure_executable` and exercised in
  `integration-tests/tests/scenario_c_stale_plan_blocked.rs`).
- Compensation support (`supported` / `partial` / `unsupported`) is
  declared by plugins, never inferred or upgraded by Core (ADR 0004).
- Risk is computed by a pure, deterministic function — never by a model
  call (ADR 0006).
- New plugins must document, honestly, whether they route through
  `forge_plugin::HostCapabilityGate` yet (ADR 0007 tracks the current
  gap for the two in-process plugins).

## Code style

- `cargo fmt` before committing.
- `cargo clippy --workspace --all-targets` should be clean, or any
  allowed lint should carry a comment explaining why.
- Prefer explicit error types (`thiserror`) over `anyhow` inside domain
  crates; `anyhow` is acceptable in `forge-cli` only, where errors are
  terminal (displayed to the operator, not matched on by a caller).
- A crate's public API should read as documentation on its own — module-
  and item-level doc comments are expected, not optional, for anything
  under `crates/` or `plugins/`. Explain *why*, not just *what*, whenever
  a decision isn't obvious from the code alone (see any existing file for
  the expected density of "why" comments).

## Tests

- Every new invariant gets a unit test in the crate that owns it, not
  just an integration test. Integration tests in `integration-tests/`
  are for cross-crate acceptance scenarios (see the five scenarios
  already there, mirroring Sections 26-30 of the original project brief)
  — they are not a substitute for testing a crate's own logic in
  isolation.
- If you add a new acceptance scenario, name the test file
  `scenario_<letter-or-name>_<short-description>.rs` to match the
  existing convention.

## Commit and PR expectations

- One logical change per PR. A PR that both adds a plugin and refactors
  storage internals is two PRs.
- Reference the relevant ADR/RFC/Section number in the PR description
  when applicable — reviewers should not have to reverse-engineer which
  requirement a change is satisfying.
- Update `README.md`'s status section in the same PR as the change it
  describes, not as a follow-up.

## Non-goals (do not propose these for v0.1)

Per Section 31 of the original project brief: no Kubernetes, Terraform,
AWS/GCP/Azure, cloud dashboard, web UI, authentication, billing, teams,
SaaS, remote telemetry, or AI-as-source-of-truth integration. Proposals
in this space belong in a future RFC once the Core is stable, not in a
feature PR against v0.1.
