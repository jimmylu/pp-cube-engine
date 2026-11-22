# Proposal

## Why

Presto B0 compilation now has broad T0 coverage, but recognized requests such as
`ungrouped: true` and `hour` fail loading like misspelled input. Callers cannot
distinguish unsupported Cube behavior from invalid input. The seed requires this
distinction, while main scenario B0-QUERY-008 currently contradicts it.

## What Changes

- Carry the explicitly listed unsupported query options through typed loading;
  reject them at compilation with `CompileError::Unsupported`, without SQL.
- Preserve model-validation precedence and strict rejection of unknown input.
- **BREAKING**: recognized sub-day granularities change from load failures to
  typed inputs rejected as Unsupported. Public query structs gain optional fields
  and the granularity enum gains variants; downstream exhaustive matches and
  struct literals need updates.
- Narrow B0-QUERY-008 to unknown granularities; retain its id.
- Transfer complete requirements B0-QUERY-009 and B0-AGG-003 from the seed.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `contract-query`: distinguish recognized unsupported input from unknown input
  and establish model-validation precedence for the supported carrier.
- `compile-b0`: adopt the complete B0 boundary requirement from the seed.

## Impact

B0 only. Affected implementation paths: `crates/pp-cube-model/src/query.rs`,
nearby model tests, `crates/pp-cube-compile/src/` and compile tests,
`crates/pp-cube-adapter/src/options.rs`, and any workspace query struct literals.
Planning and lifecycle paths: this folder, the seed's `contract-query` and
`compile-b0` deltas and notes, and those two main specs at final sync.
No dependency, vendor, harness, service or dialect addition is planned.
Existing successful SQL and golden snapshots must remain unchanged.

Non-goals: implement detail queries, comparisons, sub-day grouping or other B1
features; accept arbitrary Cube fields; add T1/T2 infrastructure. See
`review.md` for the overall gaps and delivery sequence.
