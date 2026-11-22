# Design

## Context

See proposal.md. The audit was run before writing this: thirty-nine cases were
compiled live and executed by `scripts/check.sh doris-container` on Doris 4.1.3
with no template change. Existing Doris goldens in `pp-cube-compile` cover
families under B0-DORIS-003, 005, 007, 008 and 009; none of them covered
notEquals, gte, lt, lte, a number dimension, a boolean filter, an or group
holding an and group, a two-member order, a plain range without granularity, or
a row-filtered source, and only four of them were ever executed.

## Goals / Non-Goals

**Goals:** one declaration per case that yields both its golden and its
execution; every B0 matrix member present.

**Non-Goals:** rewriting the existing compile goldens; removing the tier's
stored-golden assertions (harness).

## Decisions

- **Cases are declared once, in `pp-cube-conformance`.** The golden test
  snapshots the SQL and parameters of the manifest itself, so "each golden case
  runs on Doris" holds by construction. Alternative: add goldens to
  `pp-cube-compile` and repeat each query in a case, rejected because the two
  copies can drift.
- **One snapshot for all Doris cases.** A single file is reviewed as one
  document and includes the thirteen temporal cases. Eleven of those repeat SQL
  already pinned by `pp-cube-compile` goldens; the overlap is accepted to keep
  the rule "every executed case has a golden" without exceptions.
- **One retail fixture.** All fixtures create the same wide `dw.sales_dtl` and
  the two dimension tables when absent, truncate them and insert their own
  rows, so fixtures never disagree about a table's shape. The temporal fixtures
  now insert into that table by column list; their SQL is unchanged.
- **Rows that discriminate.** The fixture has a NULL region (negations keep
  it), a region made of `%`, `_` and backslash with the decoy `axxb` (a leaked
  wildcard would match it), an invalid row, two sales of one order (distinct
  count), and three stores in two regions. The query-timezone case uses a
  context zone that would select a different sale.
- **Expected results are literal rows, not a second query.** The seed wording
  compared with a hand-written reference query. Literal rows on a small fixture
  are at least as strict and cannot share a mistake with a reference query; the
  accepted B0-DORIS-010 already uses this wording. The scenario text is changed
  accordingly and flagged for the owner in the pull request.
- **Splitting.** B0-DORIS-001 and B0-DORIS-002 keep their ids and stay matrix
  scenarios; no new id is taken. The test for B0-DORIS-001 exercises every case.

## Risks / Trade-offs

- A matrix scenario claimed by one test → the manifest test pins the case
  counts per scenario, and the snapshot lists every case by name.
- The model fixture grew (joins, a primary key) → the eleven temporal SQL
  statements were compared with their `pp-cube-compile` goldens and are identical.
- Acceptance depends on a local tier → delivery runs it on every round; a
  failure leaves the pull request Draft.
- The tier runs the stored-golden assertions as well until the harness cleanup;
  both must pass.

## Migration Plan

Additive. Revert to return to thirteen cases. Evidence: `scripts/check.sh ci`,
`engines-container` and `doris-container` on the final head.
