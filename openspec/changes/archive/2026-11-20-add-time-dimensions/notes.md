# Session notes

## 2026-11-20: proposal drafted

Drafted by Claude (Anthropic) through omp, after `add-query-filters` merged as
#19. Planning only; no product code.

Done:
- Proposal, design, tasks and the `contract-query` delta.
- The seed's `contract-query` delta was re-cut in the same commit: its two
  requirements on time move here whole, with `B0-QUERY-001`, `B0-QUERY-003`,
  `B0-QUERY-004` and `B0-QUERY-008`. The sentence on accepting
  `timeDimensions` and `timezone` moves here too. `B0-QUERY-010` is new. The
  seed keeps "Checks before compilation" with `B0-QUERY-009`.

The owner said on 2026-11-20 to follow the proposal as written, as one change.
The points that were open:
- The query's timezone wins over the context's; the context's is required.
- A `dateRange` is an array of two date values without a zone; the
  single-string form is refused.
- Bounds are converted as Cube's JavaScript converts them, including around a
  clock change.
- `equals` and `notEquals` stay refused on a time dimension. The design of
  `add-query-filters` expected them to arrive with the date operators; reading
  the planner showed that it would not read their value in the effective
  timezone.
- The size, about 900 lines, or the split into two changes.

Learned, from reading:
- The planner does not refuse a relative range: it binds the text as a
  parameter. It panics on a `dateRange` that does not have two entries. Both
  have to be stopped before planning.
- Presto's time functions are in `PrestodbQuery.ts`; the conversion of bounds
  is in the shared `time.ts` and is the same for every dialect.

## 2026-11-20: implemented

By Claude (Anthropic) through omp, test first. All tasks.

Done:
- Query type: `timeDimensions`, `timezone`, the five granularities, a date
  range of two date values, the six date operators. Date values are checked
  when the query is read.
- Compile context: a required `timezone`.
- Adapter: time dimensions and the effective timezone reach the planner;
  predefined granularities resolve; the dialect's time functions render from
  `driver/` templates; bounds are moved to UTC after `localTimestampToUtc`;
  checks before planning.
- Templates added: `filters/time_range_filter`,
  `filters/time_not_in_range_filter`, `driver/time_stamp_cast`,
  `driver/time_grouped_column`, `driver/convert_tz`; the setting
  `timestamp_precision`. The planner asked for no other.
- Four golden snapshots. No existing snapshot changed. `Cargo.lock` gained
  the two edges of `pp-cube-adapter` and nothing else.

What the planner does, seen in the snapshots and by probing:
- The default order is the first time dimension with a granularity,
  ascending.
- An order names a time dimension by its member without the granularity and
  orders every granularity of it. `sales.sold_at.month` is not a member.
- An order on a time dimension that has only a date range is dropped; the
  kernel refuses it.
- A time dimension with a date range and no granularity filters and is not
  selected.
- Bounds are bound as text ending in `Z` and parsed by
  `from_iso8601_timestamp`.
- `beforeDate` and `afterOrOnDate` use the start of their day, `beforeOrOnDate`
  and `afterDate` its last millisecond.

Open: delivery. The SQL has not been executed (T1, T2); Presto's conversion
expression in particular is transcribed, not run.
