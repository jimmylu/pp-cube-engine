# Design

## What Druid 26 accepts

Established on Druid 26.0.0 before writing anything, on the retail and insurance fixtures.

| Form | Druid 26 |
| --- | --- |
| join on `a = b OR (a IS NULL AND b IS NULL)` (Cube's merge) | rejected: join condition not supported |
| `a IS NOT DISTINCT FROM b` in a join | NullPointerException |
| `UNION ALL` of aggregates in a subquery (Cube's key list, the planner's union merge) | rejected |
| a SELECT over a join or a subquery, ordered by a column other than time | rejected: a scan orders by time only |
| equality `FULL JOIN` of two aggregated subqueries, grouped | runs |
| a join key built by an expression (`CONCAT(...) = CONCAT(...)`) | runs for two plain aggregates; fails inside Calcite when a side is a projection over an aggregate or over a join |
| `INNER JOIN ... ON 1 = 1` of two one-row aggregates | runs |

## Decisions

1. **The planner's FULL JOIN merge.** The pinned planner has three ways to merge aggregates and picks by the templates a dialect has: union when `tesseract/full_key_aggregate_union_null` exists, FULL JOIN when `tesseract/join_types_full` exists, otherwise the key list. Druid gets `tesseract/join_types_full`. No planner or adapter code changes; `vendor/` is untouched.
2. **NULL keys meet in the GROUP BY, not in the join.** The join condition is plain equality (`expressions/is_not_distinct_from` renders `l = r`). A row whose key is NULL matches nothing and comes out of the FULL JOIN alone; the SELECT over the join then groups by the merged keys (`COALESCE(l.k, r.k)`) and takes `MAX` of each measure, so the two halves of a NULL key become one row. With several keys the same holds for a row with NULL in any of them. Every measure type of the model is a number, so `MAX` is defined; within a group each measure has at most one non-NULL value, so `MAX` selects, it does not aggregate.
3. **Where the template does it.** `statements/select` for Druid recognises the planner's merge SELECT by what it is: no GROUP BY of its own, a FULL JOIN in its FROM, and key columns of the form `COALESCE("q_l".…`. Those are the names the planner's `FullJoinMergeStrategy` writes at the pinned commit. If the planner changes them, the SQL loses its GROUP BY, Druid rejects it, and the goldens and the Druid tier both fail: the dependency is checked, not assumed.
4. **The outermost statement is grouped when it is ordered.** A statement with CTEs and an ORDER BY gets `GROUP BY` of all its columns. Its rows are already distinct by their keys, so the grouping changes nothing but the kind of Druid query, which may then be ordered by any column. Without an ORDER BY (totals) nothing is added, and the SQL equals Cube Core's.
5. **A rejected alternative: an exact null-safe key in the join** (`CONCAT` of a NULL flag and the value). It ran for two facts and failed for three, and for a formula measure, with an assertion inside Calcite 1.21 (`SqlToRelConverter.getRootField`): an expression in a join condition is pushed into a projection the converter then loses track of. Decision 2 has no expression in the condition.
6. **Time shifts come with it.** A shifted measure is two aggregates merged on the time dimension, with interval arithmetic the dialect has had since `add-druid-approx-and-granularities`. A shift by weeks or quarters is refused by the existing interval check.
7. **Relations to Cube Core.** Fourteen cases state `differs`; the tier verifies that Cube Core's SQL is other than the kernel's and that Druid rejects it. The two totals state nothing and must equal Cube Core.

## What a caller should know

Druid runs the merge on the broker: each aggregate is a subquery whose rows the broker holds. `druid.server.http.maxSubqueryRows` (100000 by default) bounds them; a query whose aggregates return more groups than that fails on the engine. This is a property of Druid's joins, not of this SQL, and is in the guide.

## Not in this change

- Rolling windows: the planner joins a time series to the aggregate on a range of time, and Druid joins on equality only. A cross join with a filter runs, but multiplies periods by rows on the broker; it needs measuring first.
- Ungrouped queries: a Druid scan orders by time only.
