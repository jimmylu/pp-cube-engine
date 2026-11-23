# Spec Delta

## MODIFIED Requirements

### Requirement: Doris dimension joins
Doris SHALL compile B0 joined dimensions through the existing same-source unique join graph, including joins needed only by security filters. Ordered parameters, selected columns and physical lineage SHALL be preserved. Unconnected, ambiguous and cross-source paths SHALL return query errors, for each fact of a query over several. A filter on a count over repeated rows SHALL return unsupported. Neither failure SHALL return SQL.
#### Scenario: B0-DORIS-005 Dimension join compilation
- **WHEN** a Doris request groups or filters by joined dimensions, or a security filter alone needs a two-hop dimension join
- **THEN** SQL equals its reviewed golden and parameters, columns and lineage preserve the joined dependencies
#### Scenario: B0-DORIS-006 Join boundary enforcement
- **WHEN** a Doris request uses unconnected cubes, an ambiguous path, a cross-source path, two facts with a dimension of one of them only, or a filter on a count from a cube reached through a many_to_one join
- **THEN** it fails with the declared error class without SQL

## ADDED Requirements

### Requirement: Doris queries over several facts
The Doris dialect SHALL compile every compile-b1 query over several facts to Doris SQL with its own identifier quoting and its limit and offset order, and with the same ordered parameters, columns and lineage as Presto for the same query.
#### Scenario: B1-DORIS-001 Multi-fact golden SQL
- **WHEN** two facts by a shared dimension, their totals, a filtered and paged combination, and a sum over repeated rows are compiled for Doris
- **THEN** the SQL equals its reviewed golden, and parameters, columns and lineage equal those of the Presto compilation
