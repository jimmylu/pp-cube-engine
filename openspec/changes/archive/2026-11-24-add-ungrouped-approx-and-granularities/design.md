# Design

## Context

See proposal.md. The pinned planner implements all three; the query type, the
model contract and three dialect functions stood in the way.

## What Cube Core 1.8.28 does, established first

Twenty-five queries were put to Cube Core on Presto and to the kernel, on one model
written in the kernel's contract. Every query both compile has byte-identical
SQL and equal parameters (but for Cube's default row limit).

| Query | Cube Core on Presto | Here |
| --- | --- | --- |
| ungrouped dimensions, measures, joins, filters, order, page | one SELECT, no GROUP BY, measures as row expressions | same SQL |
| ungrouped with a measure filter | error | query error |
| ungrouped with a sum over repeated rows, or a rolling measure | CTEs, some of them grouped | unsupported |
| approximate count: selected, in a formula, in a filter, beside another cube's sum | `approx_distinct(...)` | same SQL |
| granularity with an origin, several units | `date_add(unit, floor(date_diff(unit, origin, t) / n) * n, origin)` | same SQL |
| granularity of one unit with an offset or an origin | `date_trunc` around a subtracted and added interval | same SQL |
| undeclared granularity | error | query error |
| rolling window by a declared granularity | compiles | unsupported |

Checked on a Presto 0.293 container: for a six-month granularity from
2026-01-01, Cube's expression puts 2025-06-15 in the period of 2025-07-01 and
2025-12-15 in that of 2026-01-01. `date_diff` counts whole units toward zero,
so every row before the origin lands one period late.

## Decisions

- **`Granularity::Custom(String)`.** The query keeps the name; whether the
  time dimension declares it is a query rule checked against the model, as
  member names are. Text that cannot be a name still fails where the query is
  read. `Granularity` stops being `Copy`. Alternative: a separate string
  field; rejected because Cube's format has one.
- **Granularities are given by interval, origin and offset only**, one unit
  kind, day to year. Cube also allows an expression and compound intervals;
  those stay model errors.
- **Several units need an origin.** Without one the planner takes the start
  of the current year, read from the system clock, so the same query would
  compile to different SQL next year. The kernel reads no clock; validation
  requires the origin. With one unit and no origin the planner truncates
  naturally and the clock does not reach the SQL.
- **Rows before the origin are refused, not miscounted.** Where the planner
  counts periods from the origin, the date range of the time dimension must
  start at the origin or later, on both dialects. It is a general rule rather
  than a Presto one: Doris's floor functions were not examined for dates
  before their origin, and one rule is easier to state. Granularities that
  only move natural periods are unaffected.
- **No rolling by a declared granularity.** The time series of a rolling
  window would have to be aligned to the origin; the planner does it from an
  origin that, for an offset, is derived from the clock. Unsupported.
- **Ungrouped stays one SELECT.** For measures it aggregates apart, the
  planner writes common table expressions of which some are grouped; the
  result is neither rows nor totals. The kernel refuses SQL that begins with
  `WITH` for an ungrouped query, after planning: the cases are exactly those
  where the planner leaves one SELECT, which it knows and the bridge would
  have to predict.
- **Measure filters in ungrouped queries are a query error**, checked with the
  other filter rules, as Cube reports it.
- **The approximate count is a dialect function** (`driver/count_distinct_approx`).
  It may not have a rolling window: over a window Cube would count rows, an
  exact count under an approximate name.
- **Doris.** `APPROX_COUNT_DISTINCT`; `<UNIT>_FLOOR(t, n, origin)` for periods
  from an origin; `DATE_SUB`, `DATE_TRUNC`, `DATE_ADD` for moved periods.

## Risks / Trade-offs

- An unknown granularity name moves from a load error to a query error →
  specified; the message names the dimension.
- Execution cases for the approximate count rely on the engines being exact
  for three values → true of both sketches at this size; a larger case would
  need a tolerance the harness does not have.
- Rust API breaks → listed in the proposal and the changelog.
- Agreement is established where checked: 25 probes and 65 execution cases.

## Migration Plan

Additive for models and queries, but for the Rust types. Revert to restore the
refusals. Evidence: `scripts/check.sh ci`, remote `t0`, `presto-smoke`,
`doris-smoke`, `cube-core`.
