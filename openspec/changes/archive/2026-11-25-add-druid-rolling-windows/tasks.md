# Tasks

## 1. Establish

- [x] 1.1 Probe Druid 26.0.0 for a rolling window it executes with the rows of the other engines; record it in design.md.
- [x] 1.2 Ask Cube Core's Druid dialect for the SQL of the nine cases and run it on Druid.

## 2. Dialect and adapter

- [x] 2.1 `rolling::select` in the adapter and `kernel/rolling_select`, the time series and the cast in `templates/druid.yaml`; drop the refusal; test B1-DRUID-003 with a golden and a unit test; reduce B0-DRUID-002.

## 3. Evidence

- [x] 3.1 State the relation to Cube Core of the eight cases; extend the test of B0-EXEC-007; review the changed goldens.
- [x] 3.2 Run the Druid tier on real containers with the 82 cases; record it in notes.md.
- [x] 3.3 Update the guide and changelog.

## Workflow follow-up

- Archived in the same commit as the implementation.
- Druid now refuses only ungrouped queries. G4 is complete with this change.
