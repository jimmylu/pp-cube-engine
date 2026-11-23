# Notes

## 2026-11-23: investigation, design and implementation

The owner asked to continue with the next pull request of route units 6 to 8.
This is unit 6, B1 multi-fact queries; units 7 and 8 follow as their own
changes (AGENTS.md rule 8). Implemented with Claude Code (Anthropic).

Established first (design.md has the table): forty-nine queries were put to
Cube Core 1.8.28 on Presto and to the kernel with its refusals removed. Every
query both answer has byte-identical SQL and equal parameters; every query
Cube refuses is a query error here. The one deliberate difference is a filter
on a count over repeated rows, which Cube compiles and the kernel refuses.

Done:
- Measures of several cubes: the bridge validates one join tree per fact and
  one data source across facts, and lets the planner plan. The multi-stage
  backstop and the flatness prediction of the previous unit are removed.
- Four templates from the pin: `expressions/column_reference`,
  `statements/cte`, `expressions/is_null`, `tesseract/join_types_inner`.
- Sum, avg, min and max through many_to_one; a measure beside a one_to_one
  measure; a formula over measures of two cubes.
- A filter on a count over repeated rows is unsupported, by name.
- Lineage reads the aliases of the planner's key subquery.
- Seven execution cases (50 in all), with refund and visit tables, a nameless
  fourth store and a floor area in the retail fixture of both dialects.

Evidence, on the working tree that became the implementation commit:
- `check fast: PASSED — 4 steps, 0 skipped, 316s @ 0c2017b+dirty`.
- Local, on demand: `check conformance: PASSED — Cube Core 1.8.28, 50 cases:
  SQL and parameters equal the kernel's but for Cube's default row limit, and
  results equal on Presto 0.293 (B0-PRESTO-002)`; `check doris-container:
  PASSED — Doris 4.1.3, 50 live-compiled manifest cases (B0-DORIS-002,
  B0-DORIS-010, B0-DORIS-011)`. `engines-container` was not run locally; the
  conformance tier executes the same fifty cases on Presto.
- Existing cases' SQL is unchanged: the two manifest goldens gained seven
  blocks and lost no line. No existing compile golden changed.
- `check ci` and the remote checks are recorded on the pull request.
  Cross-model review: not run.

Open:
- The filtered count is upstream behaviour worth an issue: selected as
  `COUNT(DISTINCT pk)`, compared as `count(pk)`. It also compiled on main.
- A formula over one fact and a cube only another fact reaches is refused with
  the planner's generic rule message.
- Units 7 (time shift, rolling window, period to date) and 8 (views,
  ungrouped, approximate distinct, custom granularities).

Learned: the planner needed no help to write multi-fact SQL, only templates
and a bridge that stops refusing. Facts no join connects are still combined in
one statement, so "one data source" has to be checked across facts and not
per join tree; an existing precedence test caught that. Cube matches NULL
dimension keys across facts, and both engines evaluate that join as written.
