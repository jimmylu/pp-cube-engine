# model-validation Specification

## Purpose
Report structural model faults together, with stable locations and rules, before callers request compilation.

## Requirements

### Requirement: Structured complete validation
Validation SHALL return all detected issues with object path, rule id and message. It SHALL check required fields, types, reference resolution, join targets, primary keys at both join ends, one join per cube pair and the contract-model field allowlist.
#### Scenario: B0-VALID-001 Invalid definitions
- **WHEN** models independently contain missing fields, invalid types, unresolved references, absent join targets, missing join keys, duplicate joins or unknown fields
- **THEN** each is rejected with a located issue, and a model containing several such faults returns all detected issues

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
