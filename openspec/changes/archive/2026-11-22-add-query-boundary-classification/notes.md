# Session notes

## Current state

Implementation is complete, the main specs are synced, and this change is archived
at `archive/2026-11-22-add-query-boundary-classification`. The branch remains
`change/add-query-boundary-classification` for review corrections. The current
request requires leaving edits in the worktree without pushing or merging.

## 2026-11-22: overall review, implementation and archive

User confirmed #39 merged and requested overall gap analysis followed by next-PR
planning. Main assessment base: 7c20185. Planning was followed by implementation,
spec sync and archive in this change.

Done:
- Inspected implemented specs and remaining seed requirements, model/query,
  facade and dialect APIs, service/conformance stubs and tier documentation.
- Recorded concrete capability gaps, examples and a staged route in review.md.
- Selected recognized-query boundary classification; documented alternatives,
  error precedence, public source breaks and complete matrix acceptance.
- Transferred whole seed requirements B0-QUERY-009 and B0-AGG-003; main specs
  were subsequently synced with the implemented delta.
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
- Delivery evidence for the corrected head must be established in a future
  authorized delivery session; this record does not claim Ready status.

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
- Main specs are synced and the change folder is archived. Archive status alone
  does not establish exact-head delivery evidence or owner approval.

## 2026-11-22: review corrections

Done:
- Reproduced the unknown-granularity diagnostic defect with a tightened
  B0-QUERY-008 model test: the derived error advertised all eight enum variants.
- Added a granularity reader that names the five B0 units for unknown names,
  while preserving second/minute/hour loading and compile-time Unsupported.
- Strengthened model and compile tests and corrected the query module and enum
  documentation to describe the recognized unsupported carrier.
- Reconciled tasks and notes with the implemented, synced and archived state.

Verification at a1d38ee+dirty:
- Focused model query and compile time suites: 13 and 16 tests passed.
- `openspec validate --all --strict`: 10 passed, 0 failed.
- `scripts/check.sh ci`: controller tests, vendor/upstream alignment, lockfile,
  formatting, Clippy and all workspace tests passed before the deny step failed.
  Last check line: `check ci: FAILED at "deny" — see the advisories, bans, licences or sources above`.
  Cargo deny could not lock its advisory database on a read-only path. The full
  command output is in `/tmp/pp-cube-review-corrections-ci.log` for this session.
  Later CI steps were not run by that invocation; no CI pass is claimed.

Open: rerun the unchanged CI command with a writable advisory database cache
before delivery, then establish review and remote evidence for the final head.
No commit, push, merge, gate or snapshot change is authorized by this request.

Learned: a supported-unit substring assertion also matched the misleading
eight-variant error. Tests must identify the B0 rule explicitly.
