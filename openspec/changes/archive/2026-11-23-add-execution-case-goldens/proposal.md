# Execution case goldens

## Why

Execution cases are compiled live, so nothing in review shows when the SQL an
engine tier executes has changed. Eleven of the thirteen Doris cases repeat SQL
pinned elsewhere; the DST and security cases, and every future case, are not
pinned at all.

## What Changes

- One reviewed golden per dialect holds the SQL and ordered parameters of every
  execution case. A changed or new case fails the test until its golden is reviewed.
- `pp-cube-conformance` takes `insta` as a dev-dependency at the workspace's
  existing pin; `Cargo.lock` gains that one edge.
- Baseline level: B0. No generated SQL, manifest content or existing golden changes.

Non-goals: new cases (the Doris matrix cases follow in `add-doris-b0-acceptance`,
which was split here because its review packet exceeded the controller's limit
once `Cargo.lock` changed); harness paths; B0-DORIS-001 and B0-DORIS-002, which
stay proposed in the seed.

## Capabilities

### New Capabilities

None.

### Modified Capabilities
- `execution-cases`: adds the requirement that every execution case has a reviewed golden.

## Impact

Paths: `crates/pp-cube-conformance/Cargo.toml`, its `tests/manifest.rs` and new
`tests/snapshots/`, `Cargo.lock`, `openspec/specs/execution-cases` and this
change folder. Observable output: none.
