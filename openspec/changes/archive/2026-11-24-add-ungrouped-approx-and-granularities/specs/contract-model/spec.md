# Spec Delta

## MODIFIED Requirements

### Requirement: Dimension and measure fields
The kernel SHALL accept exactly the fields below, plus the optional metadata on both and format on measures. It SHALL preserve `meta` without interpreting it.
- dimension: required name, sql, type (string, number, boolean, time); primary_key; granularities
- measure: required name, type (count, count_distinct, count_distinct_approx, sum, avg, min, max, number); sql, required unless count; filters; rolling_window; multi_stage; time_shift
#### Scenario: B0-MODEL-001 Equivalent model formats
- **WHEN** equivalent YAML and JSON models define every dimension type, measure type and optional field above
- **THEN** both load with equivalent definitions and unchanged metadata

## ADDED Requirements

### Requirement: Declared granularities and approximate counts
A time dimension MAY declare `granularities`, each with a name, an interval and optionally an origin or an offset, and a measure MAY have the type `count_distinct_approx`. They SHALL load as written from YAML and JSON. A dimension without granularities SHALL serialize without the field, so the content digest of a model that declares none is unchanged.
#### Scenario: B1-MODEL-002 Granularities and approximate count load
- **WHEN** a model declares a granularity with an origin, one with an offset and an approximate distinct count, in YAML and again as JSON
- **THEN** each loads with its fields as written, both formats give the same model, and a dimension without granularities serializes without the field
