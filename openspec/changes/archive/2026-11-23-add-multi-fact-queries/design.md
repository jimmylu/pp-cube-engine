# Design

## Context

See proposal.md. The planner at the pin already writes multi-fact SQL; on main
the bridge refused the query before the planner saw it, refused the SQL when it
came back with common table expressions, and four templates were missing.

## What Cube Core 1.8.28 does, established first

Forty-nine queries were put to Cube Core on Presto and to the kernel with the
refusals removed. For the first ten Cube's default planner and
`CUBEJS_TESSERACT_SQL_PLANNER=true` gave identical SQL, as for the joins of the
previous unit; the rest used the default.

| Query | Cube Core | Here |
| --- | --- | --- |
| two or three facts over a shared dimension | one CTE per fact, joined to the distinct union of keys with NULL-safe equality | same SQL |
| facts without a dimension | one-row CTEs, `INNER JOIN ... ON 1 = 1` | same SQL |
| dimension filter, segment, time dimension on a shared cube | inside every CTE; parameters repeated per CTE | same SQL |
| measure filter, also on an unselected measure or a third fact | `WHERE` on the combined rows | same SQL |
| order, limit, offset | on the combined rows | same SQL |
| sum, avg, min, max through many_to_one | CTE over a subquery of distinct keys | same SQL |
| root measure with a one_to_one measure; formula over both | flat | same SQL |
| formula over a repeated sum and a root count | one CTE each, formula outside | same SQL |
| a fact's own dimension, time dimension or filter with another fact | error: no join path | query error |
| filter on a many_to_one count | flat, `HAVING count(pk)` | unsupported |

Every query Cube answered and the kernel answers has byte-identical SQL and
equal parameters (but for Cube's default row limit). Every query Cube refused
is a query error here.

## Decisions

- **Let the planner plan; validate what it will ask for.** The bridge no longer
  predicts flatness. It groups what the query reads into facts (a cube whose
  measures are named, in measures or filters, with every cube those measures
  read) and requires each fact to have one tree of joins to the dimensions,
  filters and segments of the query. That is the tree the planner asks for, so
  a missing or ambiguous path is reported by name before planning.
  Alternative: remove the check and rely on the planner's requests; rejected
  because its errors do not name the rule.
- **One data source for the whole query.** Facts no join connects are still
  combined in one statement, so the rule that held per tree now holds across
  facts. Found by an existing precedence test, not by design.
- **No backstop on multi-stage SQL.** The previous unit refused any SQL with a
  `WITH`. That is now the product. What bounds the output instead is the
  feature boundary (time shifts, rolling windows and the rest are still
  refused before planning) and the Cube Core comparison on every execution case.
- **A filter on a count over repeated rows is unsupported.** Selected, such a
  count is `COUNT(DISTINCT pk)`; in a filter the planner writes `count(pk)`
  over the joined rows, and Cube Core emits the same. The filter then disagrees
  with the column it is read against: a store counted once passes
  `store_count > 1` when it has two sales. This shape compiled on main since
  the previous unit; the wider reach of measure filters here makes it worth
  refusing rather than repeating. The rule is conservative: it also refuses the
  variant where another measure forces a CTE and the count would have been
  distinct. Alternative: follow Cube; rejected because the kernel would return
  rows its own columns contradict. Reported as an issue, not hidden.
- **Lineage reads the key subquery's aliases.** There the planner aliases a
  cube `<fact>_key_<cube>`. A table alias counts for a cube when it is the
  cube's alias or that form with a cube of the model as `<fact>`.
- **Execution cases stay in the existing acceptance scenarios.** B0-PRESTO-003,
  B0-PRESTO-002 and B0-DORIS-002 speak of every execution case; the new cases
  join the same group rather than duplicating those requirements under B1 ids.
- **Scenario ids start a B1 series** (`B1-MULTI-*`, `B1-DORIS-*`) in a new
  `compile-b1` capability, leaving `compile-b0` as the record of B0.

## Risks / Trade-offs

- SQL may begin with `WITH` → stated in the proposal and the guide; the
  compile result is still one statement.
- One compiled shape becomes unsupported → specified, tested, in the changelog.
- A formula over a fact and a cube only another fact reaches is refused with
  the planner's generic rule message, not a named one → query error class is
  right; wording can follow.
- Doris has no Cube Core dialect → accepted by execution of the same seven
  cases, including NULL-key matching and paging.
- Agreement is established where checked: 49 probes and 50 execution cases.

## Migration Plan

Additive but for the filtered count. Revert to restore the refusals. Evidence:
`scripts/check.sh ci`, remote `t0`, `presto-smoke`, `doris-smoke`, `cube-core`.
