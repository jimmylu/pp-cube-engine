# Tasks

## 1. Establish

- [x] 1.1 Probe Druid 26.0.0 for a merge of aggregates it executes; record what it accepts and rejects in design.md.
- [x] 1.2 Ask Cube Core's Druid dialect for the SQL of the sixteen cases and run it on Druid.

## 2. Dialect

- [x] 2.1 Templates that choose the planner's FULL JOIN merge and complete it with the GROUP BY; drop the two refusals; test B1-DRUID-002 with a golden; reduce B0-DRUID-002.

## 3. Evidence

- [x] 3.1 State the relation to Cube Core of the fourteen cases; extend the test of B0-EXEC-007; review the changed goldens.
- [x] 3.2 Run the Druid tier on real containers with the 73 cases; record it in notes.md.
- [x] 3.3 Update the guide and changelog.

## Workflow follow-up

- Archived in the same commit as the implementation.
- Rolling windows on Druid need a measured design; see design.md.
