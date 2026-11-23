# Joined measures and third-cube joins

## Why

The kernel refused two things Cube does, and one question about the join bridge
was open (issues #23, #22 and #30). With the Cube Core comparison in place they
could be settled against Cube itself instead of by argument.

## What Changes

- **Measures of a joined cube** (#23). The measures of a query may belong to a
  cube reached through joins. Through one_to_one joins every measure type
  compiles flat. Through a many_to_one join, count and count_distinct compile
  as Cube writes them; sum, avg, min and max need multi-stage SQL and return
  unsupported, naming the measure. Before, every such measure was a query error.
- **Join conditions that read a third cube** (#22). The cubes a join condition
  reads are added and joined before that join, as Cube does. Before, a query
  needing such a join was refused. Conditions that cannot be ordered are still
  a query error.
- **Speculative join lookups** (#30). Established from the pinned planner: a
  lookup that finds no path is an answer the planner acts on, and no call site
  swallows an error. The existing behaviour is correct and is now specified and
  tested.
- A compilation the planner can only write as multi-stage SQL is unsupported.
- Three execution cases cover the new SQL on Presto, on Doris and against Cube
  Core; B0-PRESTO-002 leaves the seed now that the comparison runs on GitHub.
- Baseline level: B0. New goldens only; no existing golden changes.

Non-goals: measures of more than one cube in one query (Cube writes
`sales.amount` with `receipts.tax` flat; here it stays unsupported, as part of
multi-fact work); multi-stage SQL for sums through many_to_one (B1); Druid.

## Capabilities

### New Capabilities

None.

### Modified Capabilities
- `compile-b0`: join requirements relaxed (B0-JOIN-004, B0-JOIN-005), third-cube joins (B0-JOIN-007), joined measures (B0-JOIN-006), speculative lookups (B0-JOIN-008).
- `dialect-doris`: join boundary reworded (B0-DORIS-006).
- `dialect-presto`: adds the Cube Core comparison (B0-PRESTO-002).

## Impact

Paths: `crates/pp-cube-adapter` (`joins.rs`, `result.rs`), `crates/pp-cube-compile`
tests and fixture, `crates/pp-cube-conformance` (model, fixtures, cases,
goldens), `docs/guide.md`, `openspec/changes/seed-baseline-specs/`, three specs
and this change folder. Callers: queries that failed with a query error for a
joined measure or a third-cube join now return SQL or unsupported.
