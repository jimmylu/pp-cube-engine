# Design

## Context

See proposal.md. Second product half of the semantic-assurance unit.

## What Cube Core 1.8.28 does, established first

Thirteen queries were put to Cube Core on Presto, under its default planner
and again with `CUBEJS_TESSERACT_SQL_PLANNER=true`. Both gave identical SQL for
all thirteen, so the earlier open question (which planner answers) does not
affect these results.

| Query | Cube Core | Here |
| --- | --- | --- |
| one_to_one: sum, avg, count, number formula | flat SELECT | same SQL |
| many_to_one: count, count_distinct | flat; count is `COUNT(DISTINCT pk)` | same SQL |
| many_to_one: sum, avg, min, max | a CTE over distinct keys | unsupported |
| a root measure with a one_to_one measure | flat | unsupported (two cubes' measures) |
| join condition reading a third cube | third cube joined first | same SQL |

After the change the kernel's SQL was byte-identical to Cube's in every row
marked "same SQL".

## Decisions

- **Let the planner decide flatness, and predict it for a good message.** The
  pinned planner already writes these measures correctly once the bridge stops
  refusing them. The bridge now names the measure when it can see the result
  would not be flat (an additive measure of a cube whose rows a many_to_one
  join repeats). As a backstop, SQL that comes back with common table
  expressions is unsupported: the baseline is one flat SELECT whatever caused
  the planner to leave it. Alternative: an allowlist only, rejected because a
  case it missed would emit multi-stage SQL silently.
- **min and max through many_to_one are unsupported**, although they are
  mathematically safe under repeated rows: Cube itself writes them as a CTE,
  and following Cube beats being clever.
- **Third cubes: close over the conditions, then order.** The tree is rebuilt
  with the cubes its join conditions read until nothing is added, then joins
  are ordered so each comes after its source cube and after every cube its
  condition reads. This is the loop Cube runs in its JavaScript layer; the Rust
  planner does not, which is why the bridge has to. A cycle is a query error.
  The added cubes pass the existing rules: one path, one data source.
- **Speculative lookups (#30): no code change.** Traced at the pin: the planner
  asks through `try_join_tree_for_hints` in one place and treats `None` as "no
  path, try another grouping"; every call site propagates errors with `?`. So
  a missing path must not latch a failure (it does not) and a refusal must
  survive a later success (it does). A regression now pins both.
- **Measures of two cubes stay unsupported.** Cube writes a root measure beside
  a one_to_one measure flat; allowing it means deciding which combinations stay
  flat across several cubes, which is the multi-fact work that follows.

## Risks / Trade-offs

- Queries that were a query error are now unsupported or succeed → an
  intended, specified change; no SQL that compiled before changes.
- The flatness prediction and the planner could disagree → the backstop turns
  any multi-stage result into unsupported rather than emitting it.
- Thirteen probes and three execution cases: agreement where checked.

## Migration Plan

Additive for callers. Revert to restore the refusals. Evidence: `scripts/check.sh
ci`, remote `t0`, `presto-smoke`, `doris-smoke`, `cube-core`.
