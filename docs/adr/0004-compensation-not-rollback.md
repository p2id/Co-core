# ADR 0004: "Compensation" Is a Distinct Concept From "Rollback"

## Status
Accepted

## Context
"Rollback" implies a guaranteed, symmetric inverse operation. Many real
operations (sending an email, deleting a resource, an external side effect)
have no true inverse. Calling everything "rollback" creates false confidence
during incidents.

## Decision
- The domain type is `Compensation`, not `Rollback`, anywhere in Core,
  storage, CLI output, and docs.
- Every `Action` declares a `CompensationSupport` of
  `Supported | Partial | Unsupported` at plan-construction time, based on
  the plugin's declared capability for that operation kind — never inferred
  or assumed by Core.
- `Unsupported` compensation on a `CRITICAL`-risk action forces
  `Policy::Decision::RequireApproval` before execution (Section 30).

## Consequences
- CLI output must never print the word "rollback" for an operation whose
  `CompensationSupport` is not `Supported`.
- This is checked by a CLI snapshot test that fails if "rollback" appears in
  any user-facing string.
