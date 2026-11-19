## Purpose
Define the Cube-compatible model inputs accepted by the B0 compile kernel.
## ADDED Requirements
### Requirement: Cube fields
The kernel SHALL accept exactly these cube fields, in YAML or equivalent JSON: name, sql_table, sql, data_source, sql_alias, joins, dimensions, measures, segments, and the optional metadata title, description, public and meta. A cube MUST have a name, a data_source and exactly one of sql_table or sql.
#### Scenario: B0-MODEL-002 Source selection
- **WHEN** a cube omits its name or data_source, or supplies both or neither of sql_table and sql
- **THEN** validation reports a model issue at the offending cube path
### Requirement: Immutable model bundles
A model bundle SHALL be a set of models with a version and a caller-supplied digest. The kernel SHALL never mutate a bundle; a new version replaces it as a whole.
#### Scenario: B0-MODEL-003 Bundle replacement
- **WHEN** two bundle versions that define the same cube differently are compiled alternately
- **THEN** each result reflects its own bundle's definition and neither bundle changes
