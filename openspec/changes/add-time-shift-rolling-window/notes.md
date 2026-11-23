# Notes

## 2026-11-24: investigation, design and implementation

The owner asked to continue with the next pull request after #82 merged.
Seventh unit of the route: B1 rolling windows, period to date and time
shifts. Implemented with Claude Code (Anthropic).

Established first (design.md has the table): fifty-three queries were put to
Cube Core 1.8.28 on Presto and to the kernel. What both compile is
byte-identical SQL with equal parameters. On a Presto 0.293 container,
`interval '1' week` and `interval '1' quarter` are syntax errors, so part of
what Cube writes for Presto does not run.

Done:
- Model: `rolling_window`, `multi_stage`, `time_shift` on measures, validated
  to a narrow form and not serialized when absent (digests unchanged).
- Bridge: the fields are handed to the planner; date ranges of time
  dimensions are completed as Cube's API layer completes them.
- Dialects: interval functions as templates, the interval units each engine
  has, a generated time series for Presto and an enumerated one for Doris.
- Refusals by name in `windows.rs`: no time dimension; a shift along a time
  dimension the query lacks; on Presto weeks, quarters and a rolling range
  that starts inside a period; on Doris rolling by a granularity without a range.
- Nine execution cases (59 in all), on the existing fixtures.

Evidence, on the working tree that became the implementation commit:
- Local, on demand: `check conformance: PASSED — Cube Core 1.8.28, 59 cases:
  SQL and parameters equal the kernel's but for Cube's default row limit, and
  results equal on Presto 0.293 (B0-PRESTO-002)`; `check doris-container:
  PASSED — Doris 4.1.3, 59 live-compiled manifest cases (B0-DORIS-002,
  B0-DORIS-010, B0-DORIS-011)`. Both passed on their first run.
  `engines-container` was not run locally.
- Existing SQL is unchanged: the manifest goldens gained nine blocks and lost
  no line; no existing compile golden changed, including after the date-range
  completion.
- `check fast`, `check ci` and the remote checks are recorded on the pull
  request. Cross-model review: not run.

Open:
- Three things Cube compiles and the kernel refuses are owner-visible choices,
  listed in the pull request: no time dimension, weeks and quarters on Presto,
  an unaligned rolling range on Presto.
- Doris enumerates the series; a year by day is 365 lines of SQL.
- Unit 8: views, ungrouped queries, approximate distinct counts, custom
  granularities.

Learned: the planner builds a rolling time series from the date-range text it
is given, while it completes bare dates itself when filtering; the bridge has
to complete them the way Cube's API layer does. Cube's generated series on
Presto starts at the range start, not at a period start. An average or a
distinct count with a window is computed over rows, so they are correct.
