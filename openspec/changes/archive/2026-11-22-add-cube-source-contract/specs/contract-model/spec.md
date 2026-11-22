## ADDED Requirements
### Requirement: Cube fields
The kernel SHALL accept exactly these cube fields, in YAML or equivalent JSON: name, sql_table, sql, data_source, sql_alias, joins, dimensions, measures, segments, and the optional metadata title, description, public and meta. A cube MUST have a name, a data_source and exactly one of sql_table or sql.
#### Scenario: B0-MODEL-002 Source selection
- **WHEN** a cube omits its name or data_source, or supplies both or neither of sql_table and sql
- **THEN** validation reports a model issue at the offending cube path
#### Scenario: B0-MODEL-006 Cube field boundary
- **WHEN** equivalent YAML and JSON documents supply every optional cube field with either sql_table or sql, and another document adds an unknown cube field
- **THEN** both valid forms load equivalently with their fields preserved and validate without issues, while the unknown field produces a model issue at its field path
