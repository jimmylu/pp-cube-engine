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

Open, for the owner before implementation:
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
