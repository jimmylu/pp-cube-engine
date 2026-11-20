# Session notes

## 2026-11-20: proposal drafted

Drafted by Claude (Anthropic) through omp. Planning only; no product code.

Done:
- Proposal, design, tasks and the `contract-query` delta.
- The seed's `contract-query` delta was re-cut in the same commit so that the
  parts without time can be adopted: `B0-QUERY-005` and `B0-QUERY-002` moved
  here; `B0-QUERY-001` stays in the seed for date operators and granularities;
  `B0-QUERY-006` to `B0-QUERY-009` are new. proposal.md has the table.

The owner said to continue on 2026-11-20, with the re-cut of the seed, string
values and the fit rule as proposed.

Asked and answered in this session: the seed cannot be archived yet. A normal
archive would put its 27 remaining requirements into `openspec/specs/` and fail
the traceability step for 35 scenarios; archiving without the spec update would
pass but hide the backlog. It stays active until its last requirement moves.

Learned: the planner also knows `in`, `notIn` and `onTheDate`; the baseline's
operator list is narrower than what the planner would take.

## 2026-11-20: implemented

By Claude (Anthropic) through omp, test first. All tasks.

Done:
- Query type: filters as conditions and nested groups, segments, order in
  either of Cube's forms, limit and offset.
- Adapter: filters, order, limit, offset and segments reach the planner; a
  segment definition; checks before planning.
- Presto templates the planner asked for: `filters/equals`, `not_equals`,
  `or_is_null_check`, `set_where`, `not_set_where`, `in`, `not_in`, `gt`, `gte`,
  `lt`, `lte`, `like_pattern`, `like_escape_char`, `always_true`;
  `tesseract/ilike`, `bool_param_cast`, `number_param_cast`;
  `expressions/wrap_segment_filter`.
- Seven golden snapshots; `Cargo.lock` is unchanged.

What the planner does, seen in the snapshots:
- `equals` with several values is `IN`; `notEquals` and the negative string
  operators also keep rows where the column is null.
- String operators compare in lower case and escape `%` and `_` in the value.
- Every value is bound as text; number and boolean members cast the
  placeholder in the SQL.
- Presto writes `OFFSET` before `LIMIT`.

Not as proposed, see design.md and the delta:
- `equals` and `notEquals` are refused on a time dimension.
- An order on a member the query does not select is refused.
- Values are not checked against the member's type.

## 2026-11-20: review, round 1

Codex (OpenAI) found one P2: an order written empty was passed on as absent,
so the planner put its default order back. Confirmed in the planner
(`compile_order_by` keeps `Some(empty)`). Fixed: the query type keeps absent
and empty apart; tests for both forms, loading and compiling; the requirement
says so.

Open: delivery. The SQL has not been executed (T1, T2).
