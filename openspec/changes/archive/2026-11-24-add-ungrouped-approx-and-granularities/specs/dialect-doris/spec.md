# Spec Delta

## ADDED Requirements

### Requirement: Doris rows, estimates and declared periods
Doris SHALL compile ungrouped queries, approximate distinct counts with APPROX_COUNT_DISTINCT, and declared granularities with its floor functions from an origin or with DATE_ADD and DATE_SUB around a truncation. Parameters, columns and lineage SHALL equal those of Presto for the same query.
#### Scenario: B1-DORIS-003 Rows, estimates and periods on Doris
- **WHEN** ungrouped rows with and without a join and a page, an approximate count with a formula and a filter, and granularities with an origin and with an offset are compiled for Doris
- **THEN** the SQL equals its reviewed golden, and parameters, columns and lineage equal those of the Presto compilation
