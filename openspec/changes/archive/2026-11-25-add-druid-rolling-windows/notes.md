# Notes

## Done

- Druid compiles rolling windows, period to date and running totals: nine more shared cases, 82 of 85. The three it refuses are ungrouped queries.
- `crates/pp-cube-adapter/src/rolling.rs` writes the planner's rolling SELECT in two steps for a dialect that has `kernel/rolling_select`; only Druid has it.
- Eight cases state `cube_core: differs`. In all, 24 state `differs` and 3 `refuses`; 55 equal Cube Core's SQL.
- G4, B1 on Druid, is complete with this change.

## Evidence

- Real containers, local, before delivery: `check druid-container: PASSED — Druid 26.0.0, 82 live-compiled manifest cases (B0-DRUID-004); Cube Core 1.8.28 Druid SQL and parameters equal the kernel's but for Cube's default row limit in 55 cases; Cube Core refuses 3 and writes SQL Druid rejects in 24 (B0-DRUID-003)`, 7m12s.
- The remote `druid-smoke` run on the delivered head is the evidence of record; see the pull request.

## Learned

- A left join on a range is its matches plus the unmatched left rows, and each half has a form Druid runs: a filtered cross join, and an equality join back to the series.
- Conditional aggregation looked simpler and is wrong with other dimensions: it returns a row for every period and dimension value.
- The planner hands a template the join and its condition as one string, so a template alone could not take them apart. The adapter's render hook sees the same context and can.
- Cube Core's Druid time series does not parse on Druid 26, so every rolling case with a granularity states `differs`; one without a granularity has no series and equals Cube Core.

## Open

- The cost of the cross join on Druid's broker grows with periods times aggregate rows. It is documented, not bounded by the kernel. A refusal above a size the kernel can see (the number of periods) would be a possible later guard; the number of aggregate rows it cannot see.
- No shared execution case rolls by another dimension on Druid.
- Ungrouped queries on Druid.
