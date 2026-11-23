# Spec Delta

## MODIFIED Requirements

### Requirement: Doris dimension joins
Doris SHALL compile B0 joined dimensions through the existing same-source unique join graph, including joins needed only by security filters. Ordered parameters, selected columns and physical lineage SHALL be preserved. Unconnected, ambiguous and cross-source paths SHALL return query errors. Measures from multiple cubes, and a measure that needs multi-stage SQL, SHALL return unsupported. Neither failure SHALL return SQL.
#### Scenario: B0-DORIS-005 Dimension join compilation
- **WHEN** a Doris request groups or filters by joined dimensions, or a security filter alone needs a two-hop dimension join
- **THEN** SQL equals its reviewed golden and parameters, columns and lineage preserve the joined dependencies
#### Scenario: B0-DORIS-006 Join boundary enforcement
- **WHEN** a Doris request uses unconnected cubes, an ambiguous path, a cross-source path, measures from multiple cubes, or a sum from a cube reached through a many_to_one join
- **THEN** it fails with the declared error class without SQL
