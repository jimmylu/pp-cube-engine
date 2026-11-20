# Proposal

## Why
The kernel compiles queries over one cube. A query that groups or filters a fact by a dimension of another cube, the most common shape after a plain aggregation, is refused with "a query over more than one cube is not available". The model already declares joins; nothing compiles them. Security filters on a joined cube, later, need the same join path.

## What Changes
- `pp-cube-adapter`: a join graph over the model's joins. For the cubes of a query it finds the one cube that reaches every other one, and the join path to each. The planner renders each join as a `LEFT JOIN` in the same flat `SELECT`.
- Refuse, without choosing: cubes connected by more than one eligible join path, and cubes on more than one data source.
- Fail without SQL: cubes that no join path connects, and a measure of a cube that is reached through a join.
- `templates/presto.yaml`: the join templates.
- Baseline level B0. New golden snapshots for the adopted scenarios, Presto. No existing snapshot changes.

### How the seed's `compile-b0` is re-cut
No scenario id is renumbered or reused.

| Seed today | After this change |
| --- | --- |
| "Dimension joins", `B0-JOIN-001`, `B0-JOIN-002`, `B0-JOIN-003` | Moves here whole. One sentence and the new `B0-JOIN-004` are added for cubes without a join path and for a measure of a joined cube. |
| "B0 boundary", `B0-AGG-003` | Unchanged, in the seed. |

## Capabilities
### New Capabilities
None.
### Modified Capabilities
- `compile-b0`: gains dimension joins. Its requirement on single-cube aggregation is unchanged.

## Impact
- `crates/pp-cube-adapter/**`: the join graph, join definitions, checks before planning. `crates/pp-cube-compile/**`: tests, a fixture and snapshots. `templates/presto.yaml`.
- `openspec/changes/seed-baseline-specs/specs/compile-b0/spec.md`: re-cut as above.
- No new dependency.
- Snapshots under `crates/pp-cube-compile/tests/snapshots/`; the gate labels them `risk:golden`.
- Size: an estimated 550 lines without snapshots, above the 400-line guide.

## Out of Scope
Measures from more than one cube and measures of a joined cube: both need the planner's multi-stage SQL, which is B1; they are refused here as query failures until `compile-result` brings the `unsupported` class that `B0-AGG-003` asks for. `one_to_many` joins, join paths written in a query, views. Model validation of joins: a primary key on both ends, one join per pair of cubes. Security filters from the compile context. Doris and Druid. Execution of the generated SQL. Harness paths and `vendor/`.
