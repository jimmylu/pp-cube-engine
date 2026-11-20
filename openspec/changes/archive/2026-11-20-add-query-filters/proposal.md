# Proposal

## Why
The kernel compiles a single-cube aggregation but a query can only name measures and dimensions. Without filters it cannot answer a real question, and every remaining B0 requirement that produces SQL needs them: joins are specified as "grouped and filtered", security filters are filters, and parameters do not exist until a filter binds one.

## What Changes
- `pp-cube-model`: the query type gains `filters` (conditions and nested `and`/`or` groups), `segments`, `order`, `limit` and `offset`, in Cube's query format.
- Compile filters on dimensions to `WHERE` and on measures to `HAVING`, for the fourteen operators that take plain values. Values are bound as parameters, in the order of their placeholders.
- Compile segments, explicit order, limit and offset.
- Refuse, before planning, a filter on an unknown member, an operator the baseline does not have, and an operator that does not fit the member's type.
- `templates/presto.yaml`: the templates these paths render.
- Re-cut the seed's `contract-query` delta so this change can adopt the parts that do not need time dimensions; see below.
- Baseline level B0. New golden snapshots for the adopted scenarios, Presto.

### How the seed's `contract-query` is re-cut
The seed has three requirements there, and two of them mix filters with time. No scenario id is renumbered or reused.

| Seed today | After this change |
| --- | --- |
| "B0 query vocabulary", `B0-QUERY-005` | `B0-QUERY-005` moves here under "Filter grouping". The sentences on validation order and unsupported features stay in the seed as "Checks before compilation", with the new `B0-QUERY-009`. |
| "Filter operators and granularities", `B0-QUERY-001` | Split, as the seed design allows for a matrix scenario. `B0-QUERY-001` stays in the seed for granularities and the six date operators. New here: `B0-QUERY-006` for the fourteen value operators, `B0-QUERY-007` for segment, order, limit and offset. |
| `B0-QUERY-002` Invalid query | Moves here for unknown members, unrecognized operators and incompatible types. The unrecognized granularity stays in the seed as the new `B0-QUERY-008`. |
| "Absolute closed time ranges", `B0-QUERY-003`, `B0-QUERY-004` | Unchanged, in the seed. |

## Capabilities
### New Capabilities
- `contract-query`: the query inputs the kernel accepts. This change brings filters, their grouping, segments, order and row limits, and the refusal of invalid filters; time dimensions stay in the seed.
### Modified Capabilities
None.

## Impact
- `crates/pp-cube-model/**`: query types and their loading. `crates/pp-cube-adapter/**`: query options, segments, filter checks. `crates/pp-cube-compile/**`: tests and snapshots. `templates/presto.yaml`.
- `openspec/changes/seed-baseline-specs/specs/contract-query/spec.md`: re-cut as above.
- No new dependency is expected; if one is needed, its effect on the planner's build is checked first.
- Snapshots under `crates/pp-cube-compile/tests/snapshots/`; the gate labels them `risk:golden`.
- Size: an estimated 1,000 lines without snapshots.

## Out of Scope
Time dimensions, granularities, the six date operators, time ranges and timezone. Joins, so a filter on another cube's member is refused. Security filters from the compile context. The four-way error classification, model validation, result metadata. Doris and Druid. Execution of the generated SQL. Harness paths and `vendor/`.
