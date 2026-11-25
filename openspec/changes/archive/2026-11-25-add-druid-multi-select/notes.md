# Notes

## Done

- Druid compiles queries of several SELECT statements: several facts over shared dimensions, sums over repeated rows, time shifts. Sixteen more shared cases, 73 of 85. Templates only.
- Fourteen cases state `cube_core: differs`; the two totals equal Cube Core's SQL. With the five of the change before, sixteen state `differs` and three `refuses`.
- Still refused by name on Druid: ungrouped queries and rolling windows, twelve cases.

## Evidence

- Real containers, local, before delivery: `check druid-container: PASSED — Druid 26.0.0, 73 live-compiled manifest cases (B0-DRUID-004); Cube Core 1.8.28 Druid SQL and parameters equal the kernel's but for Cube's default row limit in 54 cases; Cube Core refuses 3 and writes SQL Druid rejects in 16 (B0-DRUID-003)`, 7m10s.
- The remote `druid-smoke` run on the delivered head is the evidence of record; see the pull request.

## Learned

- The planner already has a FULL JOIN merge; a dialect opts in by having `tesseract/join_types_full`. It was the template set, not planner code, that decided the shape.
- A null-safe join is not needed to merge NULL keys. Let them miss each other in an equality FULL JOIN and meet in a GROUP BY of the coalesced keys.
- Calcite 1.21 inside Druid 26 fails with an assertion when a join condition holds an expression and a join input is a projection. Keep join conditions to column equality.
- A Druid SELECT over a join or a subquery is a scan and orders by time only; grouped, it orders by anything.
- A hidden internal error cost time twice here: the kernel's result check failed because the template had changed the outermost projection, and nothing said so. A debug switch that prints the internal cause in tests would have shown it at once.

## Open

- `maxSubqueryRows` bounds every aggregate of a merged query on Druid; whether production raises it is the owner's to say.
- Rolling windows on Druid.
- The adapter's refusal of `multi-stage` is now declared by no dialect; it stays for a dialect that needs it.

- The first delivery stopped at `typos`: the plural of the keyword SELECT reads as a misspelling. Reworded to "SELECT statements" throughout; no rule was added.
