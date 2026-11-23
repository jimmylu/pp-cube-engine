# Spec Delta

## ADDED Requirements

### Requirement: Presto windows and shifts
Presto SHALL compile rolling windows and time shifts to the SQL Cube Core writes, with a time series the engine generates from the bounds of the range. It SHALL return unsupported, naming the measure, for what that SQL cannot express on Presto: an interval in weeks or quarters, a rolling window by week or by quarter, and a rolling window whose date range does not start a period of its granularity.
#### Scenario: B1-PRESTO-001 Presto window limits
- **WHEN** Presto is asked for a rolling window by week or by quarter, a window of two weeks, a shift of a quarter, and a rolling window by month, year or day over a range that starts inside a period
- **THEN** each returns unsupported naming the measure and the limit, while ranges that start a period, a range without a granularity, a time dimension without a range and a shift over any range compile
