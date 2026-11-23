# Spec Delta

## ADDED Requirements

### Requirement: Doris windows and shifts
Doris SHALL compile rolling windows and time shifts with the periods of the time series written out one by one, aligned to the granularity, and with DATE_ADD and DATE_SUB for intervals of every unit from day to year. Columns and lineage SHALL equal those of Presto for the same query, and so SHALL the parameters of a shift. A rolling window by a granularity over a time dimension without a date range SHALL return unsupported, naming the measure.
#### Scenario: B1-DORIS-002 Windows and shifts on Doris
- **WHEN** a trailing window with a month to date by day, a two-week window by week and a prior-quarter shift by quarter are compiled for Doris, and a rolling window is requested by day without a date range
- **THEN** the SQL of the first three equals its reviewed golden, the last returns unsupported naming the measure, and columns and lineage equal Presto's where both compile
