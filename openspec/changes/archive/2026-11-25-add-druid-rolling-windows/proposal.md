# Proposal

## Why
Rolling windows and period-to-date measures are refused on Druid, a production engine. The property-insurance scenario needs them there: premium year to date, earned premium over twelve months. Cube Core's own Druid dialect writes SQL for them that Druid cannot parse.

## What Changes
- The Druid dialect compiles rolling windows: trailing and leading windows, period to date and running totals, by a time granularity over a date range, with or without other dimensions.
- The planner joins the time series to the aggregate on a range of time; Druid joins on equality only. For a dialect that asks for it, the adapter writes the planner's rolling SELECT in two steps Druid runs, with the same rows.
- Druid writes its time series out period by period, as Doris does, so rolling by a granularity needs a date range.
- The dialect no longer declares rolling windows unsupported. Only ungrouped queries stay refused by name.
- Nine more shared cases run on Druid, 82 of 85. Eight state that Cube Core's SQL differs and fails on Druid.

## Capabilities
### New Capabilities
None.
### Modified Capabilities
- `dialect-druid`: compiles rolling windows; one declared refusal left.

## Impact
`crates/pp-cube-adapter` (one new module, one hook where a SELECT is rendered), `templates/druid.yaml`, `crates/pp-cube-conformance` (relations, goldens), tests, guide and changelog. `vendor/` is untouched. Golden snapshots change: the Druid manifest gains nine cases and the support list loses nine refusals. Presto and Doris output is unchanged: the hook applies only to a dialect that has the kernel's rolling template.
