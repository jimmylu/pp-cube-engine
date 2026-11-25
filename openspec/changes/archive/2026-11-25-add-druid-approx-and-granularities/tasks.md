# Tasks

## 1. Establish

- [x] 1.1 Ask Cube Core's Druid dialect for the SQL of the five cases; run candidates on Druid 26.0.0; record in design.md.

## 2. Dialect

- [x] 2.2 Refuse a granularity offset in a unit the dialect's interval literal lacks (Druid and Presto: week, quarter).
- [x] 2.1 Templates for the approximate count, the period floor and interval arithmetic; drop the two refusals; test B1-DRUID-001 with a golden; reduce B0-DRUID-002.

## 3. Evidence

- [x] 3.1 State each case's relation to Cube Core in the Druid sources manifest; test B0-EXEC-007; review the changed goldens.
- [x] 3.2 Run the Druid tier on real containers with the 57 cases; record it in notes.md.
- [x] 3.3 Move B0-DRUID-003 and B0-DRUID-004 out of the seed change and archive it; update the guide and changelog.

## Workflow follow-up

- Archived in the same commit as the implementation.
- Next on Druid: more than one SELECT (several facts, repeated rows, time shifts), by rewriting the final SELECT; see design.md.
- A harness change may drop the stale reference to the seed change's `design.md` in `scripts/dev`.
