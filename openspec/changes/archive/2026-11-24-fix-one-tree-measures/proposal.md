# One join tree serves every fact of a query

## Why

Issue #88, found by the property-insurance scenario: outstanding reserves by
line of business was a query error, where Cube Core compiles it. The check
added with multi-fact queries required each cube that owns a measure to reach
the query's dimensions on its own. A cube joined one_to_one declares no join,
so it reaches nothing alone, although the cube that joins it reaches
everything. A basic claims report was refused.

## What Changes

- When one join tree holds every cube a query reads, a fact need not reach
  the dimensions by itself. The planner joins from that tree, as Cube does.
  Before: query error.
- A fact no cube of the query joins is still a query error, as in Cube.
- The scenario's support matrix moves two queries from refused to compiled,
  and both become execution cases.
- Baseline level: B1. One golden changes, the matrix, because the delta says
  these queries now compile; the manifest goldens gain blocks.

Non-goals: #89 and #90; any other refusal of the matrix.

## Capabilities

### New Capabilities

None.

### Modified Capabilities
- `compile-b1`: fact boundaries (B1-MULTI-005, B1-MULTI-008).

## Impact

Paths: `crates/pp-cube-adapter/src/joins.rs`, tests of `pp-cube-compile`,
`crates/pp-cube-conformance` (cases, goldens), `README.md`, `docs/guide.md`,
one spec and this change folder. Callers: queries that were a query error
now return SQL. No SQL that compiled before changes. Closes #88.
