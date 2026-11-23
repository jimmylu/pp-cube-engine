# Notes

## 2026-11-23: found by review of the Doris tier cleanup

The review of #67 (which removes the tier's stored-golden assertions) reported
that one of them had no execution case: `OFFSET` past the last group returns no
rows. This change adds that case so the cleanup loses no coverage. Implemented
with Claude Code (Anthropic). No design.md: one case, no new decision.

Done: `page_past_last_group` (B0-DORIS-011), its golden block, and the manifest
test, which now expects no rows for exactly this case. Forty cases.

Local results on the working tree that became the implementation commit:
- `check doris-container: PASSED — Doris 4.1.3, 19 stored-golden assertions and 40 live-compiled manifest cases; not full T2 acceptance`
- `check engines-container: PASSED — Presto 0.293, aggregate, text binding and Tokyo day goldens, 3 cases; not full T2 acceptance`
- `check ci: PASSED — 15 steps, 0 skipped, 185s @ f10a7b0+dirty`
Exact-head evidence is posted on the pull request by delivery.

## 2026-11-23: order reversed, #67 merged first

The owner merged the tier cleanup (#67) before this change, so on main the
Doris tier already runs only the manifest and briefly has no empty-page
execution; this change restores it. Main is merged into this branch. The first
delivery attempt (CI passed at 21629a1) was stopped because the base had moved.
proposal.md and tasks.md still describe #67 as following this change.

Open: Presto tier migration (#61).
