# Session notes

## 2026-11-20: proposal drafted

Drafted by Claude (Anthropic) through omp. Planning only; no product code.

Done:
- Proposal, design, tasks and the `contract-query` delta.
- The seed's `contract-query` delta was re-cut in the same commit so that the
  parts without time can be adopted: `B0-QUERY-005` and `B0-QUERY-002` moved
  here; `B0-QUERY-001` stays in the seed for date operators and granularities;
  `B0-QUERY-006` to `B0-QUERY-009` are new. proposal.md has the table.

Open, for the owner before implementation:
- The re-cut of the seed's requirements and the four new scenario ids.
- Filter values are strings only, as in Cube's query format.
- Operator and type fit follows Cube's documentation and is checked before
  planning, because the planner does not check it.

Asked and answered in this session: the seed cannot be archived yet. A normal
archive would put its 27 remaining requirements into `openspec/specs/` and fail
the traceability step for 35 scenarios; archiving without the spec update would
pass but hide the backlog. It stays active until its last requirement moves.

Learned: the planner also knows `in`, `notIn` and `onTheDate`; the baseline's
operator list is narrower than what the planner would take.
