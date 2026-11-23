# Notes

## 2026-11-24: investigation, design and implementation

The owner asked to continue with the next pull request after #84 merged.
Eighth unit of the route, without views: ungrouped queries, approximate
distinct counts and declared granularities. Views are split off as the next
change (see proposal.md); that is this session's reading of rule 8, for the
owner to confirm or overrule. Implemented with Claude Code (Anthropic).

Established first (design.md has the table): twenty-five queries were put to
Cube Core 1.8.28 on Presto and to the kernel. What both compile is
byte-identical SQL with equal parameters.

Done:
- Query: `Granularity::Custom`, `ungrouped: true` compiled.
- Model: `granularities` on time dimensions, `count_distinct_approx`.
- Bridge: the planner resolves declared granularities from the model; the
  ungrouped flag is passed on; two dialect functions (`count_distinct_approx`,
  `date_bin`) are templates.
- Refusals: measure filters and grouped parts in ungrouped queries; undeclared
  granularities; rolling by a declared granularity; a range that can reach
  before an origin; several units without an origin (model error).
- Six execution cases (65 in all), on the existing fixtures.

Evidence, on the working tree that became the implementation commit:
- Local, on demand: `check conformance: PASSED — Cube Core 1.8.28, 65 cases:
  SQL and parameters equal the kernel's but for Cube's default row limit, and
  results equal on Presto 0.293 (B0-PRESTO-002)`; `check doris-container:
  PASSED — Doris 4.1.3, 65 live-compiled manifest cases (B0-DORIS-002,
  B0-DORIS-010, B0-DORIS-011)`. Both passed on their first run.
  `engines-container` was not run locally.
- Existing SQL is unchanged: the manifest goldens gained six blocks and lost
  no line; no existing compile golden changed.
- `check fast: PASSED — 4 steps, 0 skipped, 499s @ 7453433+dirty`; after
  archiving, `check ci: PASSED — 15 steps, 0 skipped, 493s @ 290ea31+dirty`.
  The first two runs after archiving failed at the `openspec` step: the synced
  "Date operators and granularities" requirement was over the length limit;
  its wording was shortened, nothing else.
- Remote checks are recorded on the pull request. Cross-model review: not run.

Open:
- Views.
- Whether Doris's floor functions are right for dates before their origin was
  not examined; the range rule makes it moot.
- Two untracked scratch test files that this session did not write were in
  `crates/pp-cube-compile/tests/` (`zz_probe2.rs`, `zz_probe3.rs`, one of
  them failing on a malformed query). They were moved out of the repository,
  not deleted, and are not part of this change.

Learned: the planner's default origin for a granularity is the start of the
current year, read from the system clock; it reaches the SQL only for
intervals of several units. Cube's Presto expression for periods from an
origin is wrong for rows before the origin, because `date_diff` counts whole
units toward zero.
