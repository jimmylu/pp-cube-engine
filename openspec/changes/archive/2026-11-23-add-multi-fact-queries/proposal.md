# Multi-fact queries

## Why

A query could take its measures from one cube only. Everything else Cube
answers with more than one aggregation was unsupported: measures of two fact
cubes over a shared dimension, a sum from a cube whose rows a join repeats, a
formula over measures of two cubes. This is the sixth unit of the
productization route the owner confirmed (`archive/2026-11-23-complete-compile-contract/review.md`)
and the first of baseline level B1.

## What Changes

- **Measures of several cubes in one query.** Each fact is aggregated on its
  own over the query's dimensions and the results are combined on those
  dimensions, NULL keys included. Without dimensions the totals stand side by
  side. Before: unsupported.
- **Filters, order and paging.** Dimension filters, segments and security
  filters restrict every fact; measure filters, order, limit and offset apply
  to the combined rows. A filter may name a measure the query does not select.
- **Measures over repeated rows.** Sum, avg, min and max of a cube reached
  through a many_to_one join are aggregated over distinct keys, as Cube does.
  Before: unsupported, naming the measure.
- **What one SELECT can hold stays in one SELECT**: a measure beside one of a
  cube joined one_to_one, and a formula over measures of both. Before:
  unsupported.
- **Boundaries.** A fact that cannot reach a dimension, filter or segment of the
  query is a query error naming the cubes, as in Cube; so is a fact on another
  data source, joined or not. A filter on a count over repeated rows is
  unsupported (see design.md): **BREAKING** for one query shape that compiled
  before, a filter on a count of a cube reached through a many_to_one join.
- The planner's multi-stage SQL is no longer refused: it is what these queries
  compile to. Four dialect templates are added from the pin.
- Seven execution cases run on Presto, on Doris and against Cube Core, with two
  more fact tables in the retail fixture.
- Baseline level: B1. New goldens only; existing goldens gain blocks and no
  existing SQL changes.

Non-goals: time shifts, rolling windows and period-to-date (unit 7); views,
ungrouped queries, approximate distinct counts and custom granularities
(unit 8); Druid, whose SQL for these queries is refused by design; changing
what Cube writes for a filtered count.

## Capabilities

### New Capabilities
- `compile-b1`: queries over several facts (B1-MULTI-001 to B1-MULTI-007).

### Modified Capabilities
- `compile-b0`: the boundary no longer lists measures of several cubes (B0-AGG-003); joined measures no longer return unsupported (B0-JOIN-006).
- `dialect-doris`: join boundary reworded (B0-DORIS-006); multi-fact SQL on Doris (B1-DORIS-001).

The two capabilities change together because the B0 refusals are exactly what
the B1 capability replaces.

## Impact

Paths: `crates/pp-cube-adapter` (`joins.rs`, `result.rs`), `templates/`,
`crates/pp-cube-compile` tests, `crates/pp-cube-conformance` (model, fixtures,
cases, goldens), `README.md`, `docs/guide.md`,
`CHANGELOG.md`, `openspec/config.yaml`, three specs and this change folder.
Callers: queries that returned unsupported now return SQL that begins with
`WITH`; a host that assumed the SQL begins with `SELECT` must not. One query
shape that compiled now returns unsupported (above).
