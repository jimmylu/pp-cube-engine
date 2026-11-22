# Execution case manifest

## Why

Engine execution tiers obtain SQL by parsing stored snapshot text, and the one
live-compiled suite (Doris temporal, B0-DORIS-010) sits among compile tests and
starts its own engine. Execution acceptance for every dialect needs one place
that compiles cases live and states their expected results.

## What Changes

- `pp-cube-conformance` gains execution cases: model, query, context, fixture
  statements and independently chosen expected columns and rows.
- A command emits one JSON manifest per dialect, compiled at the time it runs,
  with typed ordered parameters and the scenario id each case supports.
- The thirteen Doris temporal cases move into the manifest. The existing opt-in
  test and its verifier read the manifest instead of holding their own copies,
  so the explicit local Doris command keeps working for this pull request.
- Baseline level: B0. No generated SQL, golden snapshot or dependency changes.

Non-goals: changing any harness path (the tier integration is issue #59 and a
separate harness pull request); migrating the stored-golden Doris or Presto
assertions; transferring or archiving B0-DORIS-010, B0-DORIS-001 or
B0-DORIS-002, which stay proposed in `seed-baseline-specs`; executing SQL in
the kernel; Cube Core comparison.

## Capabilities

### New Capabilities
- `execution-cases`: the manifest of live-compiled cases that execution tiers run.

### Modified Capabilities

None.

## Impact

Paths: `crates/pp-cube-conformance/` (sources, tests, fixtures),
`crates/pp-cube-compile/tests/doris_time.rs`,
`crates/pp-cube-compile/tests/doris_time_engine.py`, `Cargo.lock` for workspace
members only, and this change folder. The conformance crate starts depending on
the compile facade and the model crate. Observable output: a new manifest
document; compile results are unchanged.
