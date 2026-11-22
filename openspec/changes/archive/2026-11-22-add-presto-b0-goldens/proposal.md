# Proposal

## Why

Presto already has B0 golden suites, but its proposed acceptance requirement has not been adopted. The five granularities are snapshotted together only in UTC; the non-UTC timezone snapshots cover month, leaving the explicit per-granularity acceptance target incomplete.

## What Changes

- Adopt Presto B0 compilation and B0-PRESTO-001 from the seed, preserving their text and id.
- Map every B0 matrix member to existing golden tests and add non-UTC golden cases for day, week, month, quarter and year.
- Reuse PR #34's successful-compile wrapper so new cases also compare complete results across fresh processes.
- Add reviewed snapshots for new inputs only. Existing SQL, snapshots and public APIs are expected to stay unchanged.
- Remain at B0. T1/T2 execution, new dialects, service work, input-error classification and deferred issues are non-goals.

## Capabilities

### New Capabilities

- `dialect-presto`: T0 golden acceptance for the full B0 compilation matrix. The execution acceptance requirement stays proposed in the seed.

### Modified Capabilities

None.

## Impact

Implementation scope is `crates/pp-cube-compile/tests/{aggregation,filters,joins,time}.rs`, new time snapshots in that crate's `tests/snapshots/`, and this change folder. Transfer only the Presto B0 compilation requirement out of `openspec/changes/seed-baseline-specs/specs/dialect-presto/spec.md`; eventual sync/archive adds `openspec/specs/dialect-presto/spec.md`. No production, dependency, vendor or harness changes are planned.

PR #34 is Ready but unmerged at planning time. Implementation first synchronizes its owner-merged main tip; this proposal is based on current main and does not duplicate that PR's edits. Planning and implementation continue in one Draft PR.
