# Proposal

## Why
The kernel filters, orders and limits a single-cube aggregation, but it cannot group by month or restrict a query to a period, and nearly every real query does one of the two. Time is also the last part of the query vocabulary in `contract-query`: after this change the seed keeps only the order of checks there.

## What Changes
- `pp-cube-model`: the query type gains `timeDimensions` (dimension, granularity, dateRange) and `timezone`, in Cube's query format, and the six date operators on filters. A date value is its own validated type.
- `pp-cube-compile`: the compile context gains a required `timezone`. **BREAKING** for callers that build a context; there is none outside this repository.
- Compile a granularity to a truncation of the time dimension converted into the effective timezone; compile a dateRange and the date operators to comparisons with bounds read in that timezone and bound as parameters.
- The query's timezone, when present, is the effective one; otherwise the context's. This is the precedence the seed design left to the implementing proposal.
- Refuse, before planning: a relative or malformed date range, a granularity outside the five, a date operator on a member that is not a time dimension, and an unknown timezone.
- `templates/presto.yaml`: the templates these paths render, and Presto's three time functions as templates.
- Baseline level B0. New golden snapshots for the adopted scenarios, Presto. No existing snapshot changes.

### How the seed's `contract-query` is re-cut
No scenario id is renumbered or reused.

| Seed today | After this change |
| --- | --- |
| "Date operators and granularities", `B0-QUERY-001`, `B0-QUERY-008` | Moves here whole. |
| "Absolute closed time ranges", `B0-QUERY-003`, `B0-QUERY-004` | Moves here whole. |
| "Checks before compilation", `B0-QUERY-009` | Stays in the seed. Its first sentence, on accepting `timeDimensions` and `timezone`, moves here: into the requirement above and the new "Effective timezone" with the new `B0-QUERY-010`. |

## Capabilities
### New Capabilities
None.
### Modified Capabilities
- `contract-query`: gains time dimensions with granularities, date operators, absolute closed time ranges and the effective timezone. Its four existing requirements are unchanged.

## Impact
- `crates/pp-cube-model/**`: query types and their loading. `crates/pp-cube-adapter/**`: query options, granularity resolution, dialect time functions, checks. `crates/pp-cube-compile/**`: the context, tests and snapshots. `crates/pp-cube-dialect/**`: one setting. `templates/presto.yaml`.
- `openspec/changes/seed-baseline-specs/specs/contract-query/spec.md`: re-cut as above.
- `pp-cube-adapter` gains `chrono` and `chrono-tz` as direct dependencies, at the versions the planner already resolves; `Cargo.lock` gains edges, no package.
- Snapshots under `crates/pp-cube-compile/tests/snapshots/`; the gate labels them `risk:golden`.
- Size: an estimated 900 lines without snapshots, above the 400-line guide. The alternative is two changes, granularities with timezone and then date ranges with date operators, at the cost of splitting `B0-QUERY-001` into two ids.

## Out of Scope
`equals` and `notEquals` on a time dimension, which stay refused. Relative date ranges, `compareDateRange`, custom granularities, and the granularities below a day. Rolling windows and time series. Joins, security filters from the compile context, the four-way error classification, model validation, result metadata. Doris and Druid. Execution of the generated SQL. Harness paths and `vendor/`.
