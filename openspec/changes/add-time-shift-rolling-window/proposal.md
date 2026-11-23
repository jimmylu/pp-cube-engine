# Rolling windows, period to date and time shifts

## Why

A model could not declare a measure over a window of time, and a query could
not compare a period with another. These are the measures analysts ask for
first after plain totals: the last seven days, month to date, the same month
last year. Seventh unit of the productization route
(`archive/2026-11-23-complete-compile-contract/review.md`); second of baseline
level B1.

## What Changes

- **Model.** A measure may carry `rolling_window` (trailing, leading, offset;
  or `type: to_date` with a granularity), `multi_stage`, and `time_shift`.
  Before: unknown fields, a model error. Validation holds them to a narrow
  form: whole intervals from day to year, one shift per measure, shifts and
  multi-stage on number measures only.
- **Rolling windows and period to date.** For each period of the query's time
  dimension the measure is aggregated over its window, through a time series
  of the range. Without a granularity it is one window ending with the range.
- **Time shifts.** A shifted measure is its base measures with time moved by
  the interval, combined with unshifted measures over the query's dimensions;
  formulas over both (growth) and filters on them compile.
- **Dialects.** Presto generates the time series in the engine, as Cube
  writes it. Doris writes the periods out. Each dialect declares the interval
  units it has and gets its interval functions as templates.
- **Refused, by name.** A windowed or shifted measure in a query without a
  time dimension, and a shift along a time dimension the query lacks: the
  planner would return a total over all time, or an unshifted value. On
  Presto: weeks and quarters as intervals or as the series' step, and a
  rolling range that does not start a period. On Doris: rolling by a
  granularity without a date range.
- The date range of a time dimension is handed to the planner as Cube's API
  layer hands it: a date becomes the first or last millisecond of its day.
  No existing SQL or parameter changes.
- Nine execution cases run on Presto, on Doris and against Cube Core.
- Baseline level: B1. New goldens only; no existing golden changes.

Non-goals: views, ungrouped queries, approximate distinct counts, custom
granularities (unit 8); multi-stage aggregates (`type: sum` with
`multi_stage`), `reduce_by`, `group_by`, named shifts through calendar cubes;
sub-day windows; relative date ranges; `compareDateRange`; Druid.

## Capabilities

### New Capabilities

None.

### Modified Capabilities
- `contract-model`: measure fields (B0-MODEL-001, B1-MODEL-001).
- `model-validation`: time_shift and rolling_window leave the unsupported list (B0-VALID-003); their rules (B1-VALID-001, B1-VALID-002).
- `compile-b1`: rolling windows (B1-ROLL-001 to -004), time shifts (B1-SHIFT-001, -002), the time dimension they need (B1-WIN-001).
- `dialect-presto`: what Presto cannot express (B1-PRESTO-001).
- `dialect-doris`: windows and shifts on Doris (B1-DORIS-002).

One capability, time-windowed measures, reaches from the model contract to
both dialects; it cannot be used without each of these parts.

## Impact

Paths: `crates/pp-cube-model` (`model.rs`, `validation.rs`),
`crates/pp-cube-adapter` (`schema.rs`, `tools.rs`, `options.rs`, new
`windows.rs`), `crates/pp-cube-dialect`, `templates/`, tests of the three
crates, `crates/pp-cube-conformance` (model, cases, goldens), `README.md`,
`docs/guide.md`, `CHANGELOG.md`, five specs and this change folder. Callers:
models with these fields now validate; the Rust type `Measure` gains three
public fields, which breaks code that builds one field by field.
