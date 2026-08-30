# ADR 0007: In-Process Plugins Do Not Yet Route Through the Capability Gate

## Status
Accepted (documents a known v0.1 gap; not a design decision to keep)

## Context
Section 23 and ADR 0005 require every plugin host call (filesystem write,
process spawn, network connect, etc.) to pass through
`forge_plugin::HostCapabilityGate`, which defaults to DENY. In the current
v0.1 codebase, `FilesystemPlugin` (`plugins/filesystem`) calls
`std::fs::write` directly, and `GitPlugin` (`plugins/git`) calls
`std::process::Command::new("git")` directly. Neither call is preceded by
a `HostCapabilityGate::check` call.

This is an honest architectural gap, not an oversight to hide: ADR 0005
staged the WASM sandbox for Phase 15 specifically so that Core/state-
machine work (Phases 1-11) would not block on it. The `Plugin` trait and
`HostCapabilityGate` exist today so the *interface* is stable, but the
in-process implementations built ahead of Phase 15 do not yet call
through it.

## Decision
- This gap is tracked here explicitly rather than silently accepted.
- Until Phase 15 lands, in-process plugins MUST document, in their own
  module-level doc comment, that they bypass capability enforcement (both
  `FilesystemPlugin` and `GitPlugin` already do this).
- `forge-cli` MUST NOT present `.forge/config.toml`'s `[policy]` section
  as if it is currently restricting plugin behavior — the CLI's own
  README and `--help` output should make clear that policy enforcement
  for filesystem/git operations is not yet wired end-to-end.
- No new in-process plugin (Docker, Phase 14) should be merged with a
  false claim of capability enforcement. If Docker lands before Phase 15,
  it inherits the same documented gap.

## Consequences
- Anyone deploying this v0.1 scaffold in a context where a plugin's
  capabilities need to be actually restricted (not just modeled) must
  wait for Phase 15, or manually sandbox the process running `forge`
  (container, restricted user, etc.) in the meantime.
- This ADR is the canonical place that gap is tracked; it should be
  referenced (not re-explained) anywhere else the gap comes up, and
  closed out with a follow-up ADR once Phase 15 wires the gate into both
  existing plugins.
