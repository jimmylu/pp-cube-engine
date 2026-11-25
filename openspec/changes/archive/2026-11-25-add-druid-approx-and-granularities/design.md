# Design

## What was established first

On Druid 26.0.0 with Cube Core 1.8.28, for the five shared cases involved:

| Case | Cube Core's Druid dialect | Kernel |
| --- | --- | --- |
| `approximate_distinct_count` | error: not supported by this DB (HTTP 400) | `APPROX_COUNT_DISTINCT(x)` |
| `granularity_of_two_days`, `granularity_of_two_quarters` | error: date bin not implemented (HTTP 500) | `TIME_FLOOR(t, 'P2D', TIME_PARSE(origin))` |
| `granularity_with_offset`, `uw_by_underwriting_year` | `(DATE_TRUNC('week', (t - INTERVAL 3 day)) + INTERVAL 3 day)`, which Druid rejects at parse time | the same with `INTERVAL '3' DAY` |

All five return the rows the other engines return.

## Decisions

1. **Templates only.** The planner already asks the driver for `count_distinct_approx`, `date_bin`, `add_interval` and `subtract_interval`; the Druid dialect had no templates for them and declared the features refused. The adapter gains one check, decision 4.
2. **`APPROX_COUNT_DISTINCT`, not a sketch function.** It is built into Druid; `APPROX_COUNT_DISTINCT_DS_HLL` and `_DS_THETA` need the DataSketches extension, which a production cluster may not load. All three gave the same counts on the fixtures. The query context `useApproximateCountDistinct=false` does not affect it: that setting governs `COUNT(DISTINCT)`.
3. **`TIME_FLOOR` with an ISO 8601 period and an origin** for a granularity counted from an origin. A quarter is written as three months (`P6M` for two quarters), as on Doris. The source is the time already moved to the query's zone, and the origin is a wall-clock instant, as on the other dialects.
4. **Interval literal `'3' DAY`.** Cube's parentheses are kept so the SQL differs from Cube's in the literal only. Druid's literal takes year, month, day, hour, minute and second; an offset in weeks or quarters is refused. Nothing checked the unit of an offset before: the adapter now refuses one the dialect's interval literal lacks, by name, which also closes the same gap on Presto (it wrote `interval '1' week`, which Presto rejects). Doris has both units and is unchanged.
5. **The kernel leaves Cube Core only where Cube Core does not work.** The owner's rule (2026-11-24) is that SQL may deviate where it is better. Here "better" is not a judgement: Cube Core gives no SQL, or SQL the engine rejects. Each such case says so in the sources manifest (`cube_core: refuses | differs`) and the Druid tier checks it against Cube Core and Druid on every run. A case that says nothing must still equal Cube Core's SQL.
6. **The relation is product data**, a reviewed function in `crates/pp-cube-conformance/src/druid.rs`, visible in the golden. It appears only in the sources manifest; the plain manifest is unchanged in shape.

## Not in this change

- More than one SELECT on Druid (several facts, sums over repeated rows, time shifts). Probing showed a way: Druid accepts an equality `FULL JOIN` of grouped subqueries and a null-safe key comparison when the outer query is grouped. It needs the planner's final SELECT rewritten and is a change of its own.
- Rolling windows: Druid 26 has no non-equality join; a cross join with a filter works but its cost needs measuring.
- Ungrouped queries: a Druid scan orders by time only.
- A granularity of several weeks (`P2W`) and one counted from mid-month compile and are golden-tested, and `P2W` ran in a probe, but no shared execution case covers them.
