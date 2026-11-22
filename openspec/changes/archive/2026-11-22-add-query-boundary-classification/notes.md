# Session notes

## 2026-11-22: overall review and next proposal

User confirmed #39 merged and requested overall gap analysis followed by next-PR
planning. Main assessment base: 7c20185. This session is planning only.

Done:
- Inspected implemented specs and remaining seed requirements, model/query,
  facade and dialect APIs, service/conformance stubs and tier documentation.
- Recorded concrete capability gaps, examples and a staged route in review.md.
- Selected recognized-query boundary classification; documented alternatives,
  error precedence, public source breaks and complete matrix acceptance.
- Transferred whole seed requirements B0-QUERY-009 and B0-AGG-003; main specs
  remain unchanged until meaningful implementation and final sync.
- Created proposal, two deltas, design and bounded implementation tasks.

Implementation:
- Added typed `Query.ungrouped`, `TimeDimension.compare_date_range`, and
  second/minute/hour granularities with strict unknown/malformed input handling.
- Added one shared `Query::unsupported_b0_features` predicate used by the
  facade and direct adapter entry point; model validation remains first.
- Added boundary tests for load/round-trip, public and direct planning errors,
  grouped-false SQL/parameter preservation, and the complete B0-AGG-003 matrix.
- Updated the old hour test to assert compile-time Unsupported; fortnight
  remains an input error.

Open:
- Presto T1/T2 remain unbuilt and unrun. No execution evidence is claimed.
- Different-vendor review and Ready delivery belong to the completed change,
  not this planning checkpoint.

Learned: the seed's task completion reports proposal preparation only. Main
B0-QUERY-008 and remaining seed B0-AGG-003 disagree about hour classification;
the delta explicitly resolves that contract conflict. Unknown or malformed JSON
still fails before the typed compile entry point and is outside model-first
precedence. No owner-only decision is needed for this bounded recommendation.

Verification and publication:
- `openspec validate --all --strict`: 11 passed, 0 failed, covering both active
  changes and all main specs. Planning status: all four artifacts complete.
- Implementation `scripts/check.sh ci`: PASSED — 15 steps, 0 skipped, 93s
  @ f64a5b9+dirty. The first two attempts stopped at expected formatting and
  stale hour-test/clippy issues; both were repaired before the passing run.
- Main specs were synced in this working tree; archive follows after the
  implementation commit and exact-head delivery.
