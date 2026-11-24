# Spec Delta

## MODIFIED Requirements

### Requirement: Fact boundaries
Every fact SHALL reach every cube the query's dimensions, time dimensions, filters and segments read, by one join path, unless one join tree holds every cube the query reads; and every cube of the query SHALL be on one data source whether or not a join connects them. Otherwise the kernel SHALL return a query error without SQL, naming the cubes. Model errors, unknown members and invalid security filters SHALL keep their precedence.
#### Scenario: B1-MULTI-005 Fact that cannot reach the query
- **WHEN** two facts are requested with a dimension, a time dimension or a filter of one of them only, with a dimension one of them reaches by two paths, or with one fact on another data source
- **THEN** each returns a query error without SQL, naming the cubes without a path or the ambiguity, and an invalid model, member or security filter is reported first
#### Scenario: B1-MULTI-008 One tree over the whole query
- **WHEN** a measure of a cube that declares no join is requested beside a measure of the cube that joins it one_to_one, grouped by a dimension only that cube reaches, and again without any member of the joining cube
- **THEN** the first compiles as Cube writes it, one SELECT from the joining cube, and the second returns a query error naming the cubes without a path
