# contract-model Specification

## Purpose
Define the Cube-compatible model inputs accepted by the B0 compile kernel.

## Requirements

### Requirement: Dimension and measure fields
The kernel SHALL accept exactly the fields below, plus the optional metadata on both and format on measures. It SHALL preserve `meta` without interpreting it.
- dimension: required name, sql, type (string, number, boolean, time); primary_key
- measure: required name, type (count, count_distinct, sum, avg, min, max, number); sql, required unless count; filters
#### Scenario: B0-MODEL-001 Equivalent model formats
- **WHEN** equivalent YAML and JSON models define every dimension type, measure type and optional field above
- **THEN** both load with equivalent definitions and unchanged metadata

### Requirement: Joins, segments and references
A join SHALL have exactly name, relationship and sql, and a segment exactly name and sql, all required. Expressions SHALL refer to columns and members as `{CUBE}.column`, `{member_of_this_cube}` or `{other_cube.member}`.
#### Scenario: B0-MODEL-004 Join, segment and reference forms
- **WHEN** a model declares a join, a segment and expressions using each reference form
- **THEN** each loads and resolves to its declared cube, column or member

### Requirement: Exact metadata numbers
A number in `meta` SHALL load only when it can be kept exactly: an integer within the 64-bit range, or a finite float below 2^63 in magnitude. Any other number SHALL fail to load, from YAML and from JSON alike.
#### Scenario: B0-MODEL-005 Inexact metadata number
- **WHEN** `meta` holds a non-finite number, a float of magnitude 2^63 or more, or an integer outside the 64-bit range
- **THEN** loading fails in either format, while the widest 64-bit integers load unchanged
