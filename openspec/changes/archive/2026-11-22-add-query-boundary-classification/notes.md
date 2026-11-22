# Session notes

## Current state

Implementation is complete and archived at
`archive/2026-11-22-add-query-boundary-classification`. PR #40 contains the
implementation and reviewed repairs. The final contract repair is committed in
this branch and must receive exact-head review, remote t0 and owner approval;
merge remains the owner's action.

## Delivered behavior

- `Query.ungrouped`, `TimeDimension.compare_date_range`, and recognized
  second/minute/hour granularities load as typed values.
- `Query::unsupported_b0_features` is shared by facade and adapter; model
  validation remains first. Unknown or malformed input still fails loading.
- Recognized detail/comparison/sub-day options return Unsupported without SQL;
  explicit grouped false preserves SQL and parameters.
- B0-AGG-003 now states that multi-cube Unsupported requires an otherwise valid
  query with a unique usable join tree on one data source. Invalid members,
  security filters, disconnected or ambiguous joins and differing data sources
  retain Query precedence. Existing tests cover both classes.
- Main `contract-query` and `compile-b0` specs are synced; the change is archived.

## Verification

- `openspec validate --all --strict`: 10 passed, 0 failed after the final spec
  repair.
- Boundary model, compile, adapter and result tests pass; `errors.rs` has 5
  passing tests for the multi-cube and recognized-boundary matrix.
- `scripts/check.sh ci`: passed at `defefd2` before the final documentation-only
  repair; the repair checkpoint's fast check passed at `49b201c`. A later full CI
  must bind to the final head. No T1/T2 execution evidence is claimed.

## Review history and learning

An earlier reviewer found misleading unknown-granularity diagnostics and stale
archive lifecycle notes; those were fixed and published. The next review found
that B0-AGG-003 overlapped join errors; the spec now states precedence instead of
promising Unsupported for invalid multi-cube requests. Packet-size reductions
are documentation-only and preserve this record. After-baseline issues #22, #23,
#30, #31, #32 and #36 remain deferred.
