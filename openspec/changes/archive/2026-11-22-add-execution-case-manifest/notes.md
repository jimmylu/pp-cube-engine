# Notes

## 2026-11-22: proposal

Owner agreed in conversation to create this change and its Draft PR after a
progress review; review.md holds the gaps and route. Prepared with Claude Code
(Anthropic). Proposal, spec delta, design and tasks are written; no product code
or harness path is changed and no task is started.

Open: all tasks. No owner decision blocks implementation. The order of Druid
compilation and Presto T1 in review.md is a recommendation the owner may change.

Learned: the opt-in temporal verifier starts its own container, so calling it
from the standard Doris tier would start two engines; the manifest lets the
tier run every case in one.

## 2026-11-22: implementation

Implemented with Claude Code (Anthropic). `pp-cube-conformance` now holds the
manifest library, the thirteen Doris temporal cases with literal expectations
and the `execution-cases` command. Eleven manifest cases were compared with the
stored temporal goldens: SQL and parameters are identical; no golden changed.
The opt-in test in `pp-cube-compile` and its verifier now read the manifest.

Local results on the working tree that became the implementation commit:
- `check ci: PASSED — 15 steps, 0 skipped, 246s @ 820ffda+dirty`
- `check engines-container: PASSED — Presto 0.293, aggregate, text binding and Tokyo day goldens, 3 cases; not full T2 acceptance`
- `check doris-container: PASSED — Doris 4.1.3, 19 aggregate/join/pattern/security assertions; not full T2 acceptance`
- Explicit temporal command on local Docker Doris 4.1.3:
  `Doris live-compiled manifest execution: PASSED — 13 cases`; owned container removed.
Exact-head evidence is posted on the pull request by delivery, not committed.

Open: the standard Doris tier still does not run the manifest (issue #59,
harness-only). B0-DORIS-010, 001 and 002 stay proposed in the seed. The
transitional test and verifier are deleted by the change that archives
B0-DORIS-010. Presto tier migration is issue #61.

Learned: every current `Parameter` variant has a manifest type, so the only
unrepresentable parameter is a non-finite number. Fixture statements are
idempotent (create if absent, truncate, insert) so a tier can switch fixtures
on one engine without dropping the database.
