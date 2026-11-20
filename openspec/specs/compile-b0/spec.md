# compile-b0 Specification

## Purpose
Specify the first supported query level without promising B1 behavior.

## Requirements

### Requirement: Single-cube aggregation
The kernel SHALL compile count, count_distinct, sum, avg, min, max, filtered measures and number formulas using pinned planner semantics. Direct-table B0 queries SHALL use a flat SELECT; a cube defined by row-filtered sql MAY add its source subquery.
#### Scenario: B0-AGG-001 Aggregate families
- **WHEN** each of the six aggregate types is requested over sales with a grouping dimension
- **THEN** SQL groups by that dimension and applies the matching aggregate function to the measure expression
#### Scenario: B0-AGG-002 Conditional measures and ratios
- **WHEN** sales includes a filtered sum and a number formula dividing sum by NULLIF(distinct count, 0)
- **THEN** SQL applies the measure condition inside aggregation and divides aggregated results rather than row values
