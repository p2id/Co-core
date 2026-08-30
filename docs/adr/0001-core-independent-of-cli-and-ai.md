# ADR 0001: Core Must Be Independent of CLI and AI

## Status
Accepted

## Context
Forge's long-term value is the protocol/Core, not the CLI. If Core depends on
CLI types (clap structs, terminal formatting) or on AI (LLM calls) for any
authoritative decision, the protocol becomes unusable as a library/daemon/IDE
integration later, and AI hallucination risk leaks into state truth.

## Decision
- `forge-core` and all domain crates (`forge-state`, `forge-resource`,
  `forge-change`, `forge-plan`, `forge-exec`, `forge-verify`,
  `forge-compensate`, `forge-policy`, `forge-evidence`) MUST NOT depend on
  `forge-cli` or any AI/LLM crate.
- `forge-cli` depends on Core crates, never the reverse.
- Any future `forge-ai` crate may depend on Core (read-only access to
  State/Plan/Evidence) but Core must never depend on `forge-ai`.
- This is enforced structurally: `forge-cli`'s Cargo.toml is the only
  manifest allowed to list `clap`; no core crate may list an LLM/HTTP client
  as a mandatory dependency in v0.1.

## Consequences
- Core can be embedded in a daemon, IDE plugin, or CI step without pulling in
  a terminal UI or a network client.
- AI features (Section 36 of the master context) are additive and optional,
  never load-bearing.
