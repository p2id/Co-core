# ADR 0005: Plugins Are WASM Modules Under an Explicit Capability Model

## Status
Accepted

## Context
Section 22-23 require plugins to be isolated adapters that cannot bypass
Core state/security. Native dynamic-loaded plugins (cdylib) would give
plugins full process access (arbitrary syscalls, network, filesystem)
regardless of what the user allowed.

## Decision
- Plugins compile to WASM (`wasm32-wasip1` target) and run in a sandboxed
  runtime (`wasmtime`) inside `forge-plugin`.
- The WASM host interface is a narrow set of imported functions
  (`fs_read`, `fs_write`, `proc_spawn`, `net_connect`, `docker_call`, ...),
  each gated by a `Capability` check in `forge-policy` before the host
  function executes the real syscall.
- Default capability decision is `DENY` (Section 23). A plugin's manifest
  declares the capabilities it wants; the user/policy grants them at
  `forge init` / `forge plugin trust` time — there is no implicit grant.
- v0.1 ships the Filesystem, Git, and Docker plugins as first-party WASM
  modules built from the `plugins/` workspace members, compiled with the
  same capability boundary a third-party plugin would use (no first-party
  shortcut that bypasses the sandbox).

## Consequences
- Plugin build step requires the `wasm32-wasip1` Rust target.
- Slightly higher latency per host call than a native `dylib` would have;
  acceptable trade-off for the security guarantee in Section 23.
