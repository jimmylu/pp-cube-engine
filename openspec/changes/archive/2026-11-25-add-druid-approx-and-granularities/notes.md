# Notes

## Done

- Druid compiles approximate distinct counts and declared granularities: five more shared cases, 57 of 85. Templates, and one adapter check for the unit of an offset.
- The Druid sources manifest states `cube_core: refuses` for three cases and `differs` for two. The tier that checks this was delivered first, as a harness change (#99).
- B0-DRUID-003 and B0-DRUID-004 are in `dialect-druid`; the seed change is archived.

## Evidence

- Real containers, local, before delivery: `check druid-container: PASSED — Druid 26.0.0, 57 live-compiled manifest cases (B0-DRUID-004); Cube Core 1.8.28 Druid SQL and parameters equal the kernel's but for Cube's default row limit in 52 cases; Cube Core refuses 3 and writes SQL Druid rejects in 2 (B0-DRUID-003)`.
- The remote `druid-smoke` run on the delivered head is the evidence of record; see the pull request.

## Learned

- The first checkpoint was blocked by this change's own test: an offset of `1 week` compiled to `INTERVAL '1' WEEK`, which Druid has no literal for. The unit of a granularity's offset was never checked, on Presto either. It is now refused by name where the dialect lacks the unit.

- Cube Core answers what it cannot write with different statuses: 400 for the approximate count, 500 for the date bin. The body, not the status, says it is Cube's own answer.
- Cube's Druid dialect writes `INTERVAL 3 day`. Druid 26 wants `INTERVAL '3' DAY`, with year, month, day, hour, minute or second.
- A granularity of one month counted from the 15th is written by the planner as an offset of 14 days, not as a date bin; only intervals of several units reach `TIME_FLOOR`.
- `APPROX_COUNT_DISTINCT`, `_DS_HLL` and `_DS_THETA` agree on the fixtures; the first needs no extension.

## Open

- Whether production Druid loads the DataSketches extension. If it does and tighter error bounds are wanted, the template is one line.
- Production NULL handling mode and merge buffers, as before.
- More than one SELECT, rolling windows and ungrouped queries on Druid; see design.md for what probing showed.
