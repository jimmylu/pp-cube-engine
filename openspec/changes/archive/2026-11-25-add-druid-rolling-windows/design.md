# Design

## The planner's SQL and what Druid makes of it

```sql
SELECT "time_series"."date_from" "day", sum("rolling_source"."amount") "amount"
FROM time_series AS "time_series"
LEFT JOIN cte_0 AS "rolling_source"
  ON "rolling_source"."day" > "time_series"."date_to" - INTERVAL '3' DAY
 AND "rolling_source"."day" <= "time_series"."date_to"
GROUP BY 1
```

Druid 26 rejects a join on anything but equality. Established on Druid 26.0.0:

| Form | Druid 26 |
| --- | --- |
| join on a range (`<=`, `>`) | rejected |
| `UNION ALL` of literal rows as the series (Doris's form) | rejected in a subquery |
| `VALUES` rows as the series | runs |
| the series `INNER JOIN` the aggregate `ON 1 = 1`, filtered by the range, grouped | runs |
| the series `LEFT JOIN` that grouped result on the period, by equality | runs |

## Decisions

1. **Two steps with the same rows.** A left join on a range is its matches plus the left rows without one. The matches are the series crossed with the aggregate, filtered by the range and grouped. The rows without a match come back when the series is left-joined to that result on the period:

   ```sql
   SELECT "time_series"."date_from" "day", "rolling"."amount" "amount"
   FROM time_series AS "time_series"
   LEFT JOIN (
     SELECT "time_series"."date_from" "day", sum("rolling_source"."amount") "amount"
     FROM time_series AS "time_series"
     INNER JOIN cte_0 AS "rolling_source" ON 1 = 1
     WHERE <the range>
     GROUP BY 1
   ) AS "rolling" ON "time_series"."date_from" = "rolling"."day"
   ```

   A period without rows gives its row of NULLs, as on the other engines; `running_total_by_month` expects one. With other dimensions the inner SELECT groups by them too and the outer one takes them from it, so a period without rows has NULL dimensions, again as on the other engines.
2. **A rejected alternative: conditional aggregation** (`sum(CASE WHEN <range> THEN amount END)` over the crossed rows). It keeps empty periods without the second join, but with other dimensions it returns a row for every period and dimension value, where the other engines return only those with rows in the window.
3. **In the adapter, at the one place a SELECT is rendered.** Templates alone cannot do it: the join and its condition reach the template as one string. `Templates::render_template` passes `statements/select` to `rolling::select`, which rewrites a SELECT only when the dialect has the template `kernel/rolling_select` and the FROM clause holds `LEFT JOIN … AS "rolling_source" ON …`, the alias the pinned planner gives the aggregate of a rolling window. It splits the clause into series, source and condition, marks the period columns, and renders the dialect's template. `vendor/` is untouched.
4. **It fails closed.** A rolling SELECT with a part the two-step form has no place for (a filter, HAVING, a limit, CTEs of its own, a column mixing the series with the aggregate) is an internal error, never SQL with the part dropped. The planner's `ORDER BY` is the one thing dropped: it orders the body of a CTE, which means nothing, and Druid orders a scan by time only.
5. **The series is written out** in `VALUES`, by `statements/time_series_select`, as Doris writes it in `UNION ALL`. The existing check for such dialects applies: rolling by a granularity needs a date range.
6. **Relations to Cube Core.** Eight cases state `differs`: Cube Core's series is SQL Druid cannot parse. `rolling_at_range_end` has no granularity, so no series; it equals Cube Core's SQL and states nothing.

## What a caller should know

The inner step crosses the periods with the rows of the aggregate (one per day, or per day and dimension value, of the window's range) on Druid's broker, before the filter. Thirty periods over ninety days of ten dimension values is 27,000 rows; a year by day over a year of a thousand values is 133 million and will not do. The aggregate is also a subquery under `maxSubqueryRows`. Keep the range and the dimensions of a rolling query on Druid small; it is in the guide.

## Not in this change

- Ungrouped queries on Druid: a scan orders by time only.
- No shared execution case rolls by another dimension on Druid. The form ran in a probe and is a golden of `pp-cube-compile`.
