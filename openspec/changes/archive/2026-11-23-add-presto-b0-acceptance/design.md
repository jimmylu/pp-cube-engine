# Design

## Context

See proposal.md. All forty cases were executed on real local Presto 0.293
(memory connector, session zone UTC) before this was written: thirty-eight
passed unchanged and the two paginated cases failed with "Offset support is not
enabled". With the session property set, all forty pass.

## Decisions

- **Shared declarations.** `cases.rs` holds name, query, timezone, security
  filters and expected rows once; `doris.rs` and `presto.rs` hold fixture
  statements for three named fixtures and the scenario id of each case group.
  Alternative: a second Presto case list, rejected because two lists drift and
  the point of acceptance is that engines agree on the answers. The Doris
  manifest's cases are byte-identical before and after.
- **Offset on Presto: declare, do not rewrite or refuse.** The pinned planner
  emits `OFFSET n LIMIT m`, which PrestoDB gates behind
  `offset_clause_enabled`. Options: (a) declare the property and require the
  executor to set it; (b) return unsupported for offsets on Presto; (c) rewrite
  pagination with a window function. (a) keeps pinned Cube behavior and the B0
  matrix whole and changes no output; (b) removes a matrix member and changes
  B0-PRESTO-012; (c) diverges from the unmodified planner. This change takes
  (a). It is a contract statement to callers, so the owner should confirm it at
  review; switching to (b) later is a small, separate change.
- **Session properties live in the manifest.** A top-level `session` object,
  empty for Doris. The compile result has no field for it and gains none here:
  the property depends only on the dialect, not on the query.
- **Fixtures.** The memory connector cannot truncate, so each Presto fixture
  drops and recreates its tables. Timestamps are written as `TIMESTAMP`
  literals; the tier runs with session zone UTC.
- **Scenario ids.** Every Presto case carries B0-PRESTO-003, which stays in the
  seed: a `(T2)` requirement is archived only after its tier runs it, and the
  Presto tier is a harness path. B0-PRESTO-017 is T0-observable (manifest and
  golden) and is archived here.

## Risks / Trade-offs

- Local evidence only for execution → the manifest was run with a drafted tier
  change that is not in this pull request; it is recorded as local, on-demand
  evidence and not as B0-PRESTO-003 acceptance.
- A caller that forgets the property gets an engine error, not wrong rows.
- Presto returns DOUBLE sums; expected rows are written as integers and compare
  equal in the tier.

## Migration Plan

Additive. Revert to return to Doris-only cases. Evidence: `scripts/check.sh ci`,
remote `t0`, `presto-smoke` and `doris-smoke`.
