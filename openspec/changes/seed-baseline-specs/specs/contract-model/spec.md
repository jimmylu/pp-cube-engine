## Purpose
Define the Cube-compatible model inputs accepted by the B0 compile kernel.
## ADDED Requirements
### Requirement: Cube fields
The kernel SHALL accept exactly these cube fields, in YAML or equivalent JSON: name, sql_table, sql, data_source, sql_alias, joins, dimensions, measures, segments, and the optional metadata title, description, public and meta. A cube MUST have a name, a data_source and exactly one of sql_table or sql.
#### Scenario: B0-MODEL-002 Source selection
- **WHEN** a cube omits its name or data_source, or supplies both or neither of sql_table and sql
- **THEN** validation reports a model issue at the offending cube path
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
### Requirement: Immutable model bundles
A model bundle SHALL be a set of models with a version and a caller-supplied digest. The kernel SHALL never mutate a bundle; a new version replaces it as a whole.
#### Scenario: B0-MODEL-003 Bundle replacement
- **WHEN** two bundle versions that define the same cube differently are compiled alternately
- **THEN** each result reflects its own bundle's definition and neither bundle changes
