# Proposal

## Why

Fingerprint identity has a focused fresh-process test, but the B0 golden suite
does not compare complete compilation results across processes. Planner hash
container iteration could otherwise change SQL, parameters or metadata silently.

## What Changes

- Replay each successful compilation used by every current B0 golden suite in
  four fresh test processes, comparing the complete Compiled result.
- Reuse original typed model/query/context inputs and existing golden assertions.
- Transfer Repeatable results / B0-DET-001 from seed to this change, then archive
  only with passing meaningful process comparisons.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- determinism: implement the remaining four-process B0 golden contract.

## Impact

B0 only. Scope: crates/pp-cube-compile/tests (shared process support and existing
aggregation/filters/joins/time/security suites), this folder,
openspec/changes/seed-baseline-specs/specs/determinism/spec.md for complete
requirement transfer, and openspec/specs/determinism/spec.md at archive.
No successful output, existing snapshots, production API or dependency changes.
No vendor/harness changes, SQL execution, B1 support or repairs to deferred
issues #30–#33. Process launching belongs only to tests.
