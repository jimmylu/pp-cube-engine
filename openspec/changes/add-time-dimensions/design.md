# Design

## Context
A query with `timeDimensions` or `timezone` fails to load, and the six date operators are refused when the query is read. The compile context holds only the dialect. The adapter's dialect time functions all answer "not available", and so does its granularity lookup.

Read from the pinned planner and from Cube's JavaScript at the same commit, not run:
- The planner takes `timeDimensions` as `{dimension, granularity, dateRange}` and `timezone` in the options' static data. It asserts that a `dateRange` has exactly two entries; anything else is a panic, not an error.
- A date-only start becomes `T00:00:00.000` and a date-only end `T23:59:59.999` at the dialect's precision of 3. Any other text is passed through unchanged and bound as a parameter, so the planner does not refuse `"last month"`.
- Each bound then goes through the driver's `in_db_time_zone`, and its placeholder through `time_stamp_cast`. A range renders with `filters/time_range_filter` or `filters/time_not_in_range_filter`; `beforeDate` and `afterOrOnDate` compare with the start of their value, `beforeOrOnDate` and `afterDate` with its end.
- For every granularity, predefined ones included, the planner asks the evaluator for a granularity definition, then calls the driver's `time_grouped_column` on the column wrapped by `convert_tz`. It also knows `second`, `minute` and `hour`.
- Presto (`PrestodbQuery.ts`): `timeStampCast` is `from_iso8601_timestamp(x)`, `timeGroupedColumn` is `date_trunc('<unit>', x)`, and `convertTz` adds the zone's hour and minute offset to the column and casts the result to a timestamp. `inDbTimeZone` is not Presto's own: `BaseQuery.ts` calls `localTimestampToUtc`, which returns UTC text ending in `Z`.

## Goals / Non-Goals
**Goals:** the three requirements of the delta on Presto: granularities and date operators, absolute closed ranges, and the effective timezone.

**Non-Goals:** relative ranges, custom and sub-day granularities, rolling windows; joins; filters from the compile context; classification of failures beyond query and internal.

## Decisions
- **The query's timezone wins; the context's is the default.** The context's `timezone` is a required field, as in kernel design 6.4, so there is always an effective one. Cube's query format defines `timezone` as the query's own. Alternatives rejected: refusing a query whose timezone differs from the context's, which leaves the query field able only to repeat the context; and letting the context win, which changes the meaning of a Cube query. This is the owner's to confirm.
- **Granularity is a closed enum of the five.** `second`, `minute`, `hour` and any unknown word fail when the query is read, with the position, as an operator outside the baseline does.
- **A `dateRange` is an array of exactly two date values.** Cube also takes a single string, which is either one day or a relative expression; that form is refused when the query is read. This is also what keeps the planner's assertion from being reached.
- **A date value is a type, checked when the query is read:** `YYYY-MM-DD`, or `YYYY-MM-DDTHH:mm:ss` with an optional `.SSS`, and a real calendar date. No zone suffix: the value is read in the effective timezone. Alternative rejected: passing text to the planner, which binds whatever it is given.
- **`inDateRange` and `notInDateRange` take two values; the other four date operators take one.** Checked with the type fit, which admits date operators on time dimensions only.
- **A time dimension names a dimension of type `time` and asks for a granularity, a dateRange or both.** One that asks for neither is refused; the planner would drop it without a word.
- **Bounds are converted the way Cube converts them.** `in_db_time_zone` follows `localTimestampToUtc` step for step, including its choice of offset in the hours around a clock change, so the kernel and Cube Core bind the same values. It needs `chrono` and `chrono-tz` in the adapter, at the planner's versions. Alternative rejected: resolving the local time exactly, which is more correct and differs from Cube on those hours.
- **The conversion is always written, for UTC too,** as Cube writes it when a timezone is set.
- **Presto's time functions are templates** under `driver/` in `templates/presto.yaml` (`convert_tz`, `time_grouped_column`, `time_stamp_cast`), and the timestamp precision is a setting of the data file. They are text with holes, and the dialect stays data. Alternative rejected: Rust functions per dialect, as kernel design 6.3 sketches; nothing in these three needs code.
- **`equals` and `notEquals` stay refused on a time dimension.** The planner would cast the value to a timestamp but not read it in the effective timezone, unlike the date operators. The requirement "Invalid filters" is unchanged.
- **An order may name a time dimension of the query.** Whether the planner wants the member with or without its granularity is read in the first task and recorded in notes.md.
- **Golden tests:** one snapshot with the five granularities, one with the six date operators, one for `B0-QUERY-003`, one holding both compilations of `B0-QUERY-010`. The parameters are part of each snapshot.
- **Validation:** unit tests for reading the new query fields and date values; a unit test of the bound conversion against values worked out by hand, one of them inside a clock change; the golden tests naming the five scenario ids; tests for each refusal; `scripts/check.sh ci`.

## Risks / Trade-offs
- The facts above come from reading → the first task compiles one month granularity end to end before anything else is built on them.
- Presto's conversion expression is transcribed and has never been executed → the snapshot shows it whole; execution belongs to T1 and T2.
- Around a clock change Cube's conversion is off by the size of the change → reproduced, tested and noted, not corrected.
- The root manifest patches `chrono-tz` to upstream's fork → the last task compares `Cargo.lock` with `main`: edges for the adapter only, no package added or changed.
- `B0-QUERY-001` is a matrix scenario → its tests compile all six date operators and all five granularities.
- The size is above the guide → named in proposal.md with the alternative.

## Migration Plan
The query type gains fields; a query that loads today loads unchanged. Every caller of `compile` adds a timezone to its context; in this repository those are the tests of `pp-cube-compile`. On archive, `openspec/specs/contract-query/spec.md` gains the three requirements.
