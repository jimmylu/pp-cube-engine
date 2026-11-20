# Proposal

## Why
The workspace can load a model but has never produced SQL. The public design rests on one claim: the pinned planner can be driven through its public interfaces from our own types. That was measured outside this repository and not rerun. This change makes it true here with the smallest query that exercises every layer, and gives the dialect templates, the largest unknown in the baseline, their first real data.

## What Changes
- `pp-cube-adapter`: implement the planner's bridge interfaces over the contract types, for what a single-cube query reaches: model lookups, cube, measure and dimension definitions, measure filters, expression compilation, a one-cube join tree, and the tool objects.
- `pp-cube-dialect`: a dialect is a named template set plus helper functions. Add Presto with the templates and functions the single-cube path asks for, transcribed from Cube at the pinned commit. A template or function outside that set returns an error naming it.
- `pp-cube-compile`: `compile(model, query, context)` returns SQL and ordered parameters, or a failure that separates a caller's mistake from an internal planner error.
- `pp-cube-model`: a query type with `measures` and `dimensions` only.
- Move the requirement "Single-cube aggregation" (`B0-AGG-001`, `B0-AGG-002`) out of `seed-baseline-specs` into this change.
- Baseline level B0. First golden SQL snapshots: one per scenario, Presto.

## Capabilities
### New Capabilities
- `compile-b0`: compiling B0 queries to SQL. This change brings single-cube aggregation; joins and the B0 boundary stay in the seed.
### Modified Capabilities
None.

## Impact
- `crates/pp-cube-adapter/**`, `crates/pp-cube-dialect/**`, `crates/pp-cube-compile/**`: new code and tests. `crates/pp-cube-model/**`: the query type, and the position of each reference in an expression.
- `templates/presto.yaml`: template data with the upstream file and commit it was taken from.
- `Cargo.lock`: `minijinja` becomes a direct dependency at the version and features the planner already uses; `insta` and `similar` are new, for tests only. No existing package's features change. The gate labels this `risk:upstream`; the pin does not move.
- `openspec/changes/seed-baseline-specs/specs/compile-b0/spec.md`: the moved requirement is deleted.
- Snapshots under `crates/pp-cube-compile/tests/snapshots/`; the gate labels them `risk:golden`.
- Size: about 2,000 lines without snapshots, against an estimate of 1,500. It is one behaviour, the first SQL, and no smaller part of it can be tested on its own.

## Out of Scope
Query filters, order, limit, offset, time dimensions, timezone, segments and joins. Doris and Druid. Output columns, used cubes, fingerprint and the four-way error classification of `compile-result`. Model bundles, the per-thread planner cache, the service. Execution of the generated SQL (tiers T1 and T2). Harness paths and `vendor/`.
