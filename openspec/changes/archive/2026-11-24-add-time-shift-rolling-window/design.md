# Design

## Context

See proposal.md. The pinned planner plans rolling windows and time shifts; the
contract model refused their fields, the measure bridge did not pass them on,
the dialect functions for intervals were unimplemented and the time-series
templates were missing.

## What Cube Core 1.8.28 does, established first

Fifty-three queries were put to Cube Core on Presto and to the kernel. After
the change, every query both answer has byte-identical SQL and equal
parameters (but for Cube's default row limit); the differences are the
refusals below.

| Query | Cube Core on Presto | Here |
| --- | --- | --- |
| trailing, leading, unbounded window by day, month or year | generated `time_series` CTE, source CTE, range join | same SQL |
| to_date by day or month | same, bounded from `date_trunc` of the period | same SQL |
| window without a granularity | one aggregate bounded to the window at the range's end | same SQL |
| avg, count_distinct with a window | source keeps rows; the window aggregates them | same SQL |
| window with dimensions, filters, segments, order, paging, other measures | as any query | same SQL |
| prior or next shift, growth formula, filter on a shifted measure | shifted aggregate CTE combined like a second fact | same SQL |
| window or shift without a time dimension | total over all time; unshifted value | unsupported |
| shift along a time dimension the query lacks | unshifted value | unsupported |
| window by week | `interval '1' week`, which Presto 0.293 rejects | unsupported on Presto |
| a fact's own time dimension beside another fact | error: no join path | query error |

Checked on a Presto 0.293 container: `interval '1' week` and `interval '1'
quarter` are syntax errors; month and year intervals clamp to the month's end.

## Decisions

- **Pass the fields through; let the planner plan.** As for multi-fact
  queries, the SQL is the planner's. The bridge hands over the window as
  written and a shift with its time dimension as a full path, which is what
  Cube's compiler does.
- **A narrow model form.** Intervals are one whole number of day, week, month,
  quarter or year; one shift per measure; `multi_stage` only on number
  measures; windows only on aggregates. Everything Cube allows beyond that
  (sub-day and compound intervals, multi-stage aggregates, named shifts) is a
  model error at its field, to be opened when needed. Alternative: accept
  Cube's whole grammar; rejected as untested surface.
- **New fields are not serialized when absent**, so the content digest of
  every existing model is unchanged.
- **Date ranges as Cube's API layer writes them.** The planner completes a
  bare date itself where it filters, but builds the time series from the text
  it is given. Completing the dates in the bridge makes both agree with Cube.
  Every existing golden and execution case is unchanged by it.
- **Interval functions are dialect templates** (`driver/interval_string`,
  `add_interval`, `subtract_interval`, `date_time_cast`), applied one unit at
  a time as `PrestodbQuery.ts` does. The minimal-unit function is Cube's base
  logic in Rust.
- **Each dialect declares its interval units.** Presto: year to second, no
  week, no quarter. Cube writes those units anyway and Presto rejects the
  SQL; the kernel refuses before planning and names the measure. Alternative:
  rewrite a week as seven days; rejected because the SQL would no longer be
  Cube's, and the comparison is what accepts Presto output.
- **Presto: the rolling range starts a period.** Cube's generated series
  begins at the start of the date range and steps by the granularity, so a
  range from the 15th by month yields periods from the 15th, which neither
  match `date_trunc` buckets of other measures nor mean what the column says.
  Refused by name. A range without a granularity builds no series and is
  unaffected; so is a time dimension without a range, whose series runs over
  the truncated bounds of the data.
- **Doris writes the periods out.** Cube has no Doris dialect; its MySQL
  dialect enumerates the series unless recursive CTEs are enabled. The planner
  computes the enumerated periods itself, aligned to the granularity, so Doris
  takes any range and every unit. It needs a range to enumerate: rolling by a
  granularity without one is unsupported. Timestamps are cast as the dialect's
  existing `time_stamp_cast` does.
- **Refuse what would silently be something else.** Without a time dimension
  the planner returns a windowed measure as a total over all time and a
  shifted one unshifted; a shift naming one time dimension is ignored along
  another. Each is unsupported, naming the measure. Cube compiles them.
- **The checks live in one module** (`windows.rs`), before planning, reading
  the measures a query names or computes from. As a backstop an interval unit
  the dialect lacks fails template rendering rather than emitting SQL.

## Risks / Trade-offs

- The upper bound the planner binds for a rolling source on Presto runs a day
  past the series (`…T23:59:58.999` of the next day). It is Cube's value; the
  join to the series bounds the rows, so results are unaffected, and the
  execution cases confirm it.
- Presto and Doris differ in what they accept (weeks, unaligned ranges, a
  missing range) → each difference is declared, specified and tested.
- An enumerated series grows with the range: a year by day is 365 lines of
  SQL on Doris. Acceptable for the baseline; a generated series is future work.
- `Measure` gains public fields → a source break for code constructing it.
- Agreement is established where checked: 53 probes and 59 execution cases.

## Migration Plan

Additive for models and queries. Revert to restore the refusals. Evidence:
`scripts/check.sh ci`, remote `t0`, `presto-smoke`, `doris-smoke`, `cube-core`.
