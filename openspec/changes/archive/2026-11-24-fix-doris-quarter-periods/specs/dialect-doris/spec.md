# Spec Delta

## ADDED Requirements

### Requirement: Doris quarters as months
Doris SHALL write a quarter as three months wherever its SQL counts or moves time by an interval: periods of a declared granularity counted from an origin, and the intervals of rolling windows, time shifts and granularity offsets. Doris 2.1 has neither a quarter floor function nor a quarter unit in interval arithmetic.
#### Scenario: B1-DORIS-005 Quarters on Doris
- **WHEN** a granularity of two quarters from an origin and a shift of one quarter are compiled for Doris
- **THEN** the first floors to six months from the origin and the second adds three months, and neither SQL names a quarter function or a quarter interval
