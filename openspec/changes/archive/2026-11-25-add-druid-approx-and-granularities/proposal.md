# Proposal

## Why
Druid 26.0 is a production engine and its dialect compiles only B0. Two B1 features need one SELECT, which Druid answers: approximate distinct counts and declared granularities. The property-insurance scenario uses both (an underwriting year from April; an estimated insured count). Cube Core's own Druid dialect gives neither: it refuses the first, and for the second it either refuses or writes an interval literal Druid rejects.

## What Changes
- The Druid dialect compiles `count_distinct_approx` measures and declared granularities, with templates of its own where Cube's Druid dialect has none or one that fails.
- The dialect no longer declares these two features unsupported. More than one SELECT, ungrouped queries, rolling windows and time shifts stay refused.
- The Druid sources manifest states, per case, where Cube Core does not answer with the kernel's SQL: `refuses` or `differs`. The Druid execution tier holds each to its word.
- The two Druid acceptance scenarios, B0-DRUID-003 and B0-DRUID-004, move from the seed change into `dialect-druid`: the tier that runs them exists. The seed change has transferred everything and is archived.

## Capabilities
### New Capabilities
None.
### Modified Capabilities
- `dialect-druid`: compiles approximate distinct counts and declared granularities; fewer declared refusals; acceptance by execution and by comparison with Cube Core.
- `execution-cases`: a case of a sources manifest may state its relation to Cube Core.

## Impact
`templates/druid.yaml`, `crates/pp-cube-conformance` (relation per case, goldens), tests of `pp-cube-compile`, guide and changelog. Golden snapshots change: the Druid manifest gains five cases and the support list loses five refusals. Presto and Doris output is unchanged.
