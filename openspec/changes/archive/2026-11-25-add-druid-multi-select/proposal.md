# Proposal

## Why
The Druid dialect refuses every query that needs more than one SELECT: measures of several cubes, a sum over rows a join repeats, a measure shifted in time. These are the core of the property-insurance scenario (premium beside claims, growth on the prior year), and Druid 26.0 is a production engine. Cube Core's own Druid dialect writes SQL for them that Druid rejects.

## What Changes
- The Druid dialect compiles queries of several SELECTs: measures of several cubes over shared dimensions, sums over repeated rows, time shifts and formulas over them.
- The aggregates are merged by a FULL JOIN on plain equality and a GROUP BY of the merged keys, a form Druid 26 executes. This is the planner's own FULL JOIN merge, chosen and completed by the dialect's templates.
- The dialect no longer declares more than one SELECT or time shifts unsupported. Ungrouped queries and rolling windows stay refused by name.
- Sixteen more shared cases run on Druid, 73 of 85. Fourteen state that Cube Core's SQL differs and fails on Druid; the two totals equal Cube Core's SQL.

## Capabilities
### New Capabilities
None.
### Modified Capabilities
- `dialect-druid`: compiles several SELECTs and time shifts; fewer declared refusals.

## Impact
`templates/druid.yaml`, `crates/pp-cube-conformance` (relations, goldens), tests of `pp-cube-compile`, guide and changelog. No adapter or planner code changes. Golden snapshots change: the Druid manifest gains sixteen cases and the support list loses sixteen refusals. Presto and Doris output is unchanged.
