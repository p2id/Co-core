# RFC 0001: The Forge Protocol

- **Status:** Draft
- **Author:** Forge maintainers
- **Created:** 2026
- **Supersedes:** none
- **Related:** `docs/adr/0001` through `docs/adr/0007`, `schemas/*.json`

## Abstract

This RFC defines the Forge protocol independently of any particular
implementation. It exists so that a second implementation — a different
language, a daemon instead of a CLI, an IDE plugin — could be built from
this document and `schemas/*.json` alone, without reading the Rust source.
Where this RFC and the Rust implementation disagree, that is a bug in one
of the two, and should be resolved by raising a new RFC or filing an issue
— not by treating either side as silently authoritative.

## 1. Motivation

Section "Fundamental problem" of the original project brief identifies
ten questions a change-management layer must be able to answer, from
"what is the current state?" through "what evidence proves the final
state?". Every tool in the existing ecosystem (Git, Docker, Terraform,
CI/CD systems) answers a subset of these questions for its own domain.
None answers all ten across domains. Forge's premise is that this
question set is the actual protocol, and everything else — CLI syntax,
storage engine, plugin transport — is an implementation detail that can
change without changing the protocol.

## 2. Primitives

The protocol defines thirteen primitives. Each has a canonical JSON Schema
in `schemas/`, referenced below.

| Primitive     | Schema                          | Summary                                                    |
|---------------|----------------------------------|--------------------------------------------------------------|
| Resource      | `resource.schema.json`          | Anything observable or manageable, identified by `provider://kind/id`. |
| State         | `state.schema.json`             | One of four perspectives on a Resource: OBSERVED, DESIRED, PLANNED, VERIFIED. |
| Change        | `change.schema.json`            | Immutable, normalized record of a before/after transition on a Resource. |
| Action        | `action.schema.json`            | An executable operation realizing part of a Change, with declared compensation support. |
| Plan          | `plan.schema.json`              | An ordered set of Actions with risk, constraints, and a verification/compensation strategy. Immutable after approval. |
| Execution     | `execution.schema.json`         | A record of running a Plan through its state machine to completion or compensation. |
| Evidence      | `evidence.schema.json`          | Immutable proof of any significant Core operation, queryable by Change/Plan/Execution/Resource/Action. |

Relation, Observation, Intent, Verification, Compensation, and Policy are
defined structurally in this RFC (§3-§8) rather than as standalone JSON
Schemas, because they are always embedded within, or derived from, one of
the seven schemas above rather than persisted as independent top-level
records in v0.1.

## 3. State perspectives

A protocol implementation MUST distinguish four perspectives on a
Resource's state and MUST NOT conflate them:

1. **OBSERVED** — what a plugin most recently reported as actually
   existing.
2. **DESIRED** — what a user or upstream system has declared it wants.
3. **PLANNED** — what a Plan predicts will exist once executed.
4. **VERIFIED** — what has been proven, after execution, by comparing an
   expected state to a fresh observation.

In particular: a successful command exit code is evidence toward
VERIFIED, never a substitute for it. An implementation that marks a
Resource VERIFIED without performing an explicit expected-vs-observed
comparison is non-conformant.

## 4. The core lifecycle

```
STATE -> OBSERVE -> CHANGE -> PLAN -> REVALIDATE -> EXECUTE -> VERIFY -> COMMIT
```

Failure path:

```
EXECUTE -> FAIL -> COMPENSATE -> VERIFY -> COMPENSATED
```

### 4.1 Revalidation is mandatory, not optional

Between Plan approval and Plan execution, the world may have changed. A
conformant implementation MUST re-observe the Plan's target Resources
immediately before execution and compare them against the Change records
the Plan was built from. If any target has drifted, the Plan MUST be
marked STALE and execution MUST be refused. Silently executing a stale
Plan is a protocol violation, not an acceptable optimization.

### 4.2 Verification is mandatory, not optional

An Execution MUST NOT transition to COMMITTED without an explicit
Verification step (§3) confirming that every targeted Resource now
matches its Plan's PLANNED state. An Execution whose Actions all reported
success but whose Verification step was skipped MUST NOT be reported as
COMMITTED.

## 5. Compensation is not rollback

A conformant implementation MUST expose, per Action, one of three
Compensation support levels:

- `supported` — a true inverse operation exists and will be attempted on
  failure.
- `partial` — an inverse exists but is known to be incomplete (for
  example, it cannot recover uncommitted state lost between two
  operations).
- `unsupported` — no inverse exists. This MUST be surfaced honestly; an
  implementation MUST NOT invent, assume, or silently promote
  compensation support that a plugin has not explicitly declared.

The word "rollback" implies a guaranteed, symmetric inverse and MUST NOT
be used in place of "compensation" in any protocol-conformant output.

## 6. Risk is deterministic

Risk assessment for a Plan MUST be computed by a deterministic function
of already-observed or already-declared inputs (blast radius,
environment, privilege, dependency count, compensation support). It MUST
NOT be delegated to a probabilistic model (including an LLM) as the
authoritative source in the base protocol. An AI-assisted advisory layer
MAY narrate or explain an already-computed risk value, but MUST NOT alter
it.

## 7. Plugins

A plugin adapts an external system into the primitives in §2. A plugin:

- MUST report Resources with a stable `provider://kind/id` identity.
- MUST NOT redefine State, Change, Plan, Execution, or Evidence — it may
  only produce Resources, Observations, and Relations, and execute
  Actions whose `kind` it recognizes.
- MUST declare Compensation support per Action `kind` honestly (§5),
  never speculatively.
- SHOULD run under an explicit, default-deny capability model (filesystem
  read/write, process spawn, network connect, etc.), so that a policy
  layer can restrict what any given plugin invocation is permitted to do.
  (Reference implementation status for this requirement: see ADR 0007 —
  it is a "SHOULD" the reference implementation does not yet fully meet.)

## 8. Evidence and auditability

Every Plan creation, approval, Execution state transition, Verification,
Compensation attempt, and Policy decision MUST produce an Evidence
record. Evidence records MUST be immutable once created and MUST be
queryable by the Change, Plan, Execution, Resource, or Action they
concern. An implementation MAY choose any storage strategy (event
sourcing, append-only tables plus current-state tables, etc.) as long as
this queryability and immutability guarantee holds — see ADR 0002 for the
reference implementation's specific choice, which is not itself part of
this RFC.

## 9. Non-goals of this RFC

This RFC does not specify: CLI command syntax, storage schema, network
transport for a future daemon mode, or any cloud/Kubernetes/Terraform
integration. These are implementation or future-RFC concerns.

## 10. Open questions

- Should DESIRED state be a first-class, separately-stored perspective in
  v0.1, or is it acceptable (as the reference implementation currently
  does) to approximate "desired" as "the after-state of the most recent
  Change" until a real desired-state-declaration mechanism exists?
- What is the minimum viable Relation vocabulary (§ "RELATIONS" in the
  original project brief lists eight kinds) — should implementations be
  permitted to extend it, or is it protocol-fixed?

These are left open for a future RFC rather than decided unilaterally
here.
