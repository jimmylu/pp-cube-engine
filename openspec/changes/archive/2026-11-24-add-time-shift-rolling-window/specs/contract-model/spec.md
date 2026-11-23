# Spec Delta

## MODIFIED Requirements

### Requirement: Dimension and measure fields
The kernel SHALL accept exactly the fields below, plus the optional metadata on both and format on measures. It SHALL preserve `meta` without interpreting it.
- dimension: required name, sql, type (string, number, boolean, time); primary_key
- measure: required name, type (count, count_distinct, sum, avg, min, max, number); sql, required unless count; filters; rolling_window; multi_stage; time_shift
#### Scenario: B0-MODEL-001 Equivalent model formats
- **WHEN** equivalent YAML and JSON models define every dimension type, measure type and optional field above
- **THEN** both load with equivalent definitions and unchanged metadata

## ADDED Requirements

### Requirement: Measure windows and shifts
A measure MAY carry `rolling_window`, with `trailing`, `leading` and `offset`, or with `type: to_date` and `granularity`; `multi_stage`; and `time_shift`, a list of entries with `interval`, `type` and an optional `time_dimension`. They SHALL load as written from YAML and JSON. A measure that has none of them SHALL serialize without them, so the content digest of a model that uses none is unchanged.
#### Scenario: B1-MODEL-001 Windows and shifts load
- **WHEN** a model declares a bounded window, a to_date window, a shifted multi-stage measure and a multi-stage formula over it, in YAML and again as JSON
- **THEN** each loads with its fields as written, both formats give the same model, and a measure without these fields serializes without them
