# Accept Doris temporal execution

## Why

The Doris container tier now compiles and executes the temporal cases of the
execution manifest before every review (#60, #62). B0-DORIS-010 can therefore
leave the proposal backlog, and the transitional opt-in test that started its
own engine is redundant.

## What Changes

- Transfer the complete Doris temporal execution tier requirement and
  B0-DORIS-010 from `seed-baseline-specs` to the implemented `dialect-doris`
  spec, reworded to state what the tier does instead of what was pending.
- The execution cases in `pp-cube-conformance` name B0-DORIS-010; they are the
  test of record, run by `scripts/check.sh doris-container`, not by T0.
- Delete the transitional opt-in test `real_doris_temporal_execution` and its
  verifier `doris_time_engine.py` from `pp-cube-compile`.
- Baseline level: B0. No generated SQL, golden snapshot, dependency or manifest
  content changes.

Non-goals: any harness path (a stale handoff sentence in
`docs/agents/doris-container.md` is left for the next harness pull request);
B0-DORIS-001 and B0-DORIS-002, which stay proposed in the seed; moving the
stored-golden assertions into manifest cases; full B0/T2 acceptance.

## Capabilities

### New Capabilities

None.

### Modified Capabilities
- `dialect-doris`: adds the temporal execution tier requirement with B0-DORIS-010.

## Impact

Paths: `crates/pp-cube-compile/tests/doris_time.rs`,
`crates/pp-cube-compile/tests/doris_time_engine.py` (deleted),
`crates/pp-cube-conformance/src/doris.rs` (scenario comment only),
`openspec/changes/seed-baseline-specs/specs/dialect-doris/spec.md` and its
`notes.md` (the moved requirement is removed there), `openspec/specs/dialect-doris`
and this change folder. Observable output: none.
