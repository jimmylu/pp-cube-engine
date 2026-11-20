## ADDED Requirements
### Requirement: Baseline SQL and joins
Validation SHALL restrict cube sql to one single-table row-level SELECT, forbid aggregation outside measures, permit only many_to_one or one_to_one joins, and restrict expressions to own-cube columns and declared members. SQL validation SHALL NOT rewrite the text.
#### Scenario: B0-VALID-002 Forbidden SQL constructs
- **WHEN** cube sql independently contains multiple statements, aggregation, DISTINCT, window functions, joins, subqueries, set operations or common table expressions
- **THEN** validation rejects each case at the sql field while accepting a single-table projection with a row predicate
#### Scenario: B0-VALID-003 Unsupported model features
- **WHEN** models use one_to_many, undeclared member references, time_shift, rolling_window or pre-aggregations
- **THEN** validation rejects each with the offending field or reference rather than silently ignoring it
#### Scenario: B0-VALID-004 Expression constraints
- **WHEN** a dimension or segment expression contains an aggregate function, or a member expression reaches another cube's column other than through a declared member
- **THEN** validation rejects each at the expression's field
