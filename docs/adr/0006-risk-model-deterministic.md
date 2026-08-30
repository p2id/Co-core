# ADR 0006: v0.1 Risk Scoring Is a Deterministic Rule Table, Not an LLM

## Status
Accepted

## Context
Section 20 explicitly forbids using an LLM as the authority for risk in
v0.1, and Section 2's "Critical principle" forbids AI being a source of
truth.

## Decision
Risk is computed by a pure function
`fn assess(factors: RiskFactors) -> RiskLevel` in `forge-plan`, where
`RiskFactors` is `{ blast_radius, environment, privilege, reversibility,
dependency_count }`, each already-observed/declared data — never model
output. The function is a deterministic lookup/weighting table, unit-tested
with fixed input → output pairs.

## Consequences
- Risk results are reproducible and explainable in evidence output.
- A future `forge suggest`/`forge explain` AI command (Section 36) may
  *narrate* the already-computed risk in natural language, but cannot
  change the `RiskLevel` value itself.
