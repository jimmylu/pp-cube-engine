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
- **WHEN** models use one_to_many, undeclared member references, reduce_by, case or pre-aggregations
- **THEN** validation rejects each with the offending field or reference rather than silently ignoring it
#### Scenario: B0-VALID-004 Expression constraints
- **WHEN** a dimension or segment expression contains an aggregate function, or a member expression reaches another cube's column other than through a declared member
- **THEN** validation rejects each at the expression's field

### Requirement: Validation at the compile boundary
The public compile facade SHALL validate the complete typed model before checking queries or security filters and before invoking the planner. Invalid models SHALL return a model error containing the existing deterministic validation report. The public error message SHALL NOT include SQL or physical schema details. Successful output for valid models SHALL remain unchanged.
#### Scenario: B0-VALID-005 Invalid bundles cannot bypass validation
- **WHEN** an invalid typed bundle is compiled, including with an independently invalid query or security filter
- **THEN** compilation returns its model issues before query errors, without SQL, and leaves all inputs unchanged

### Requirement: Window rules
An interval SHALL be a positive whole number of day, week, month, quarter or year; `trailing` and `leading` MAY also be `unbounded`. A rolling window SHALL have either `type: to_date` with a granularity from day to year and no bounds, or a bound with an optional `offset` of start or end, and SHALL belong to an aggregate measure that is neither multi-stage nor shifted. Each violation SHALL be reported at its field.
#### Scenario: B1-VALID-001 Window faults
- **WHEN** models independently give a window a malformed, sub-day, zero or compound interval, an unknown offset, type, granularity or field, bounds that do not go together, a number measure or a multi-stage measure
- **THEN** validation reports each at the offending field

### Requirement: Shift rules
Only a number measure SHALL be multi-stage. A time shift SHALL belong to a multi-stage measure, be the only one of its measure, have an interval and a type of prior or next, and name, if it names one, a time dimension of its own cube. Each violation SHALL be reported at its field.
#### Scenario: B1-VALID-002 Shift faults
- **WHEN** models independently give a shift an unknown type, a malformed or missing interval, an unknown field, a time dimension that is not one of its cube, a second shift or a measure that is not multi-stage, and mark an aggregate measure multi-stage
- **THEN** validation reports each at the offending field, and accepts a shift that names no time dimension
