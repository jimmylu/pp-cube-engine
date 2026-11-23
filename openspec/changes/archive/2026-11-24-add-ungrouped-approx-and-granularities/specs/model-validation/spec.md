# Spec Delta

## ADDED Requirements

### Requirement: Granularity rules
A declared granularity SHALL belong to a time dimension and have a name unique in it and not predefined, an interval of a positive whole number of day, week, month, quarter or year, and at most one of an origin (a date or timestamp) and an offset (an interval). An interval of several units SHALL have an origin: without one the pinned planner counts from the start of the current year. An approximate distinct count SHALL NOT have a rolling window.
#### Scenario: B1-VALID-003 Granularity faults
- **WHEN** models independently give a granularity a malformed or predefined name, a malformed or sub-day interval, a malformed origin or offset, an unknown field, no interval, several units without an origin, both an origin and an offset, or a repeated name, declare one on a number dimension, and give an approximate count a rolling window
- **THEN** validation reports each at the offending field, and accepts one unit with neither origin nor offset and one unit with an origin
