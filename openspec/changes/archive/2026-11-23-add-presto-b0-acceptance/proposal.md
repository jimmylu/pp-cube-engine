# Presto B0 acceptance cases

## Why

Presto compiles the B0 matrix, but only three stored goldens have ever run on a
real engine. Running a case for every matrix member on Presto 0.293 showed one
real gap: PrestoDB rejects the `OFFSET` clause the compiled SQL uses unless the
session enables it, and nothing told a caller so.

## What Changes

- The execution cases become shared declarations: one list of queries and
  expected rows, with each dialect supplying its fixtures and scenario ids.
  Presto gets the same forty cases as Doris, on memory-connector fixtures.
- The manifest states the engine session properties its SQL needs. Presto
  declares `offset_clause_enabled=true`; Doris declares none.
- One reviewed golden pins the SQL and ordered parameters of every Presto case.
- `openspec/config.yaml` drops the 400-line and two-hour rules and follows
  AGENTS.md rules 8 and 9 (a change is one capability sized by work).
- Baseline level: B0. No template, compile result or existing golden changes.
  The Doris manifest gains an empty `session` object; its cases are byte-identical.

Non-goals: harness paths. The Presto tier still executes three stored goldens;
the harness pull request that makes it run this manifest follows, and
B0-PRESTO-003 stays proposed in `seed-baseline-specs` until that tier passes
on GitHub. Cube Core comparison (B0-PRESTO-002) and Druid are separate work.

## Capabilities

### New Capabilities

None.

### Modified Capabilities
- `dialect-presto`: adds the offset session property requirement (B0-PRESTO-017).
- `execution-cases`: adds cases shared across dialects, declared session properties and the Presto golden (B0-EXEC-004).

## Impact

Paths: `crates/pp-cube-conformance/` (sources, test, new snapshot),
`openspec/config.yaml`, `openspec/changes/seed-baseline-specs/` (B0-PRESTO-003
reworded, notes), `openspec/specs/dialect-presto`, `openspec/specs/execution-cases`
and this change folder. Callers of the kernel on Presto must set
`offset_clause_enabled=true` when they pass an offset; kernel output is unchanged.
