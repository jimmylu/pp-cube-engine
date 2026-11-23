# Notes

## 2026-11-23: design and implementation

The owner asked for the next change after the lightweight-delivery harness
(#70) merged. Implemented with Claude Code (Anthropic). First change sized by
capability under AGENTS.md rule 8: Presto execution cases as one unit, with the
tier migration as its one harness follow-up.

Done: cases are shared declarations (`cases.rs`) with Doris and Presto fixtures;
the manifest declares session properties; Presto has forty cases and a reviewed
golden; B0-EXEC-004 and B0-PRESTO-017 are archived. The Doris cases are
byte-identical to before (the manifest gains an empty `session`).
`openspec/config.yaml` follows the work-sized rules.

Finding: PrestoDB 0.293 rejects the compiled `OFFSET` clause with "Offset
support is not enabled" unless the session sets `offset_clause_enabled=true`.
Chosen: declare the property in the manifest and the spec; kernel output is
unchanged. design.md lists the alternatives. **For the owner to confirm.**

Evidence:
- `check ci: PASSED — 15 steps, 0 skipped, 175s @ c532d3b+dirty` on the working
  tree that became the implementation commit. An earlier run stopped in
  `cargo deny` on a network failure fetching the advisory database.
- Local, on demand, real Presto 0.293 Docker, with a drafted tier change that is
  not in this pull request: forty of forty cases pass with the property; without
  it exactly `order_limit_offset` and `page_past_last_group` fail.
  Drafted tier line: `check engines-container: PASSED — Presto 0.293, 40
  live-compiled manifest cases (B0-PRESTO-003)`. This is development evidence,
  not B0-PRESTO-003 acceptance.

Open: harness pull request for the Presto tier (shared manifest loader, typed
parameter literals, session header, remove the three stored-golden cases and
`presto-fixture.sql`; closes #61). Then transfer B0-PRESTO-003 from the seed.
Still proposed: B0-PRESTO-002 (T1) and B0-DRUID-001 to 004.

Learned: Presto's protocol returns timestamps as `YYYY-MM-DD HH:MM:SS.mmm` text
and DOUBLE sums as JSON numbers; integer expectations compare equal. The memory
connector accepts integer literals for DOUBLE columns and NULL for VARCHAR in
`INSERT ... VALUES`.
