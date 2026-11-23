# Notes

## 2026-11-23: investigation, design and implementation

The owner asked to continue after #79 merged. Second product half of the
semantic-assurance unit: issues #22, #23 and #30. Implemented with Claude Code
(Anthropic).

Established first (design.md has the table): thirteen join queries put to Cube
Core 1.8.28 gave identical SQL under its default planner and with
`CUBEJS_TESSERACT_SQL_PLANNER=true`. That also answers, for these queries, the
earlier open question about which planner responds.

Done:
- #23: measures of a joined cube. Flat through one_to_one for every type; count
  and count_distinct through many_to_one; sum, avg, min, max through
  many_to_one are unsupported by name. Multi-stage SQL from the planner is
  unsupported as a backstop.
- #22: cubes a join condition reads are added and joined first; an unorderable
  set is a query error.
- #30: traced at the pin; no code change, behaviour specified and pinned by a
  regression (B0-JOIN-008).
- Three execution cases (43 in all), fixtures extended with receipts and
  promotions on both dialects. B0-PRESTO-002 transferred; the seed keeps only
  the paused Druid delta.

Evidence, on the working tree that became the implementation commit:
- `check ci: PASSED — 15 steps, 0 skipped, 256s @ e99057c+dirty`.
- Local, on demand: `check conformance: PASSED — Cube Core 1.8.28, 43 cases …`,
  `check engines-container: PASSED — Presto 0.293, 43 …`,
  `check doris-container: PASSED — Doris 4.1.3, 43 …`.
- The kernel's SQL for the nine "same SQL" probe queries was byte-identical to
  Cube Core's. Existing cases' SQL is unchanged; existing goldens are unchanged
  (the two manifest goldens gained three blocks; their header line changed with
  the test's expression text).
- Remote checks are recorded on the pull request. Cross-model review: not run.

Open: a root measure beside a one_to_one measure is flat in Cube and
unsupported here (multi-fact work, next). Sums through many_to_one need
multi-stage SQL (B1). The owner deferred the first version number until this
unit; it can be decided now.

Learned: Cube's default planner and its Rust planner agree on these joins. The
Rust planner writes joined measures correctly on its own; only the bridge's
refusals and the missing third-cube closure stood in the way.
