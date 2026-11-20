## Purpose
Define B0 query inputs and preserve the pinned Cube query semantics.
## ADDED Requirements
### Requirement: Value filters
The kernel SHALL accept filters in Cube's query format with the operators equals, notEquals, contains, notContains, startsWith, notStartsWith, endsWith, notEndsWith, gt, gte, lt, lte, set and notSet. A filter on a dimension SHALL restrict rows and a filter on a measure SHALL restrict groups. Filter values SHALL be bound as parameters, in the order of their placeholders in the SQL.
#### Scenario: B0-QUERY-006 Value operators
- **WHEN** queries exercise each of the fourteen operators on a member of a compatible type, on dimensions and on a measure
- **THEN** each compiles to the predicate pinned Cube semantics define for it, in WHERE for a dimension and HAVING for a measure, with its values as ordered parameters
### Requirement: Filter grouping
The kernel SHALL preserve nested and/or groups and combine top-level filters with AND.
#### Scenario: B0-QUERY-005 Filter grouping
- **WHEN** a query has two top-level filters, one of them an or group that contains an and group
- **THEN** SQL joins the top-level entries with AND and keeps each group's nesting and operator
### Requirement: Segments, order and row limits
The kernel SHALL accept segments, order, limit and offset in Cube's query format. Order SHALL name measures or dimensions of the query and SHALL be applied in the sequence it is written; an order on any other member SHALL be refused, not dropped. A query without an order SHALL get the planner's default order; an order written empty SHALL produce no ORDER BY.
#### Scenario: B0-QUERY-007 Segment, order, limit and offset
- **WHEN** a query names a segment, orders by two members in opposite directions, and sets a limit and an offset
- **THEN** SQL applies the segment's condition, orders by both members in the written sequence and direction, and carries the limit and the offset
### Requirement: Invalid filters
The kernel SHALL refuse a filter that names an unknown member, uses an operator outside the baseline, or applies an operator to a member whose type it does not fit. String operators fit string dimensions; comparison operators fit number dimensions and measures; equals and notEquals fit every member except a time dimension; set and notSet fit every member.
#### Scenario: B0-QUERY-002 Invalid query
- **WHEN** a query references an unknown member, uses an unrecognized operator, or applies an operator to an incompatible type
- **THEN** compilation returns a query error naming the member or rule
