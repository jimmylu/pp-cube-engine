# Ungrouped rows, approximate distinct counts and declared granularities

## Why

Three things a caller meets soon after totals: the rows behind a number, a
distinct count cheap enough for a large table, and periods that are not
calendar ones (a fiscal year, a fortnight). Each was refused: `ungrouped` as
unsupported, the measure type and the dimension field as model errors, the
granularity name where the query is read. Eighth unit of the productization
route, without views.

## What Changes

- **Ungrouped queries.** `ungrouped: true` returns rows: one SELECT without
  GROUP BY, measures as row expressions. Before: unsupported.
- **`count_distinct_approx`.** A measure type that compiles to the engine's
  estimate: `approx_distinct` on Presto, `APPROX_COUNT_DISTINCT` on Doris.
  Before: a model error.
- **Declared granularities.** A time dimension may declare `granularities`
  (name, interval, origin or offset) and a query may group by one by name.
  Before: a model error, and the name refused when the query is read.
- **Refused.** A measure filter in an ungrouped query (query error, as in
  Cube). An ungrouped query whose measures the planner aggregates apart. A
  rolling window by a declared granularity. An origin-counted granularity over
  a range that can reach before its origin: Cube's expression puts those rows
  in the wrong period. An interval of several units without an origin (model
  error): Cube counts it from the start of the current year, which would make
  the SQL depend on the clock.
- **BREAKING (Rust API).** `Granularity` gains `Custom(String)` and is no
  longer `Copy`; `Dimension` gains `granularities`; `MeasureType` gains
  `CountDistinctApprox`; `Query::unsupported_b0_features` is renamed
  `unsupported_features`. An unknown but well-formed granularity name is now a
  query error at compilation rather than an error when the query is read.
- Six execution cases run on Presto, on Doris and against Cube Core.
- Baseline level: B1. New goldens only; no existing golden changes.

Non-goals: **views**, the remaining part of unit 8, which extend the model
contract with a new top-level object and are their own change;
granularities given by an expression (`sql`), with compound or sub-day
intervals; HyperLogLog sketches for pre-aggregations; Druid.

## Capabilities

### New Capabilities

None.

### Modified Capabilities
- `contract-model`: dimension and measure fields (B0-MODEL-001, B1-MODEL-002).
- `contract-query`: granularity names (B0-QUERY-008, B0-QUERY-011, B1-QUERY-001).
- `model-validation`: granularity rules (B1-VALID-003).
- `compile-b0`: ungrouped leaves the boundary (B0-AGG-003).
- `compile-b1`: rows (B1-DETAIL-001, -002), estimates (B1-APPROX-001), declared periods (B1-GRAN-001, -002).
- `dialect-doris`: the three on Doris (B1-DORIS-003).

Three small query capabilities travel together because the route lists them
as one unit and each is a few lines of bridge with its own tests; views are
split off because they are not small.

## Impact

Paths: `crates/pp-cube-model` (`model.rs`, `query.rs`, `validation.rs`),
`crates/pp-cube-adapter` (`schema.rs`, `options.rs`, `tools.rs`, `windows.rs`,
`lib.rs`, new `granularities.rs`), `crates/pp-cube-compile`, `templates/`,
tests of the three crates, `crates/pp-cube-conformance` (model, cases,
goldens), `README.md`, `docs/guide.md`, `CHANGELOG.md`, six specs and this
change folder. Callers: see BREAKING above.
