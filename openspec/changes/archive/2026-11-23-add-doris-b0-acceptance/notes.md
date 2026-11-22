# Notes

## 2026-11-23: audit, proposal and implementation

Owner asked in conversation to continue with the Doris matrix after #63 merged.
Implemented with Claude Code (Anthropic).

Audit: a case for every B0 matrix member was compiled live and executed on real
local Docker Doris 4.1.3. All passed on the first run with no template change.
No Doris dialect defect was found; the gap was goldens and execution evidence.

Done: twenty-six matrix cases on one retail fixture join the thirteen temporal
cases; one golden pins SQL and parameters of all thirty-nine; the manifest test
pins the counts per scenario. B0-DORIS-001 and B0-DORIS-002 moved out of the
seed, whose Doris delta is deleted. The eleven temporal SQL statements that have
`pp-cube-compile` goldens are still identical to them.

Local results on the working tree that became the implementation commit:
- `check doris-container: PASSED — Doris 4.1.3, 19 stored-golden assertions and 39 live-compiled manifest cases; not full T2 acceptance`
- `check engines-container: PASSED — Presto 0.293, aggregate, text binding and Tokyo day goldens, 3 cases; not full T2 acceptance`
- `check ci: PASSED — 15 steps, 0 skipped, 209s @ 0af8662+dirty`
The tier's "not full T2 acceptance" wording is harness text and predates this
change. Exact-head evidence is posted on the pull request by delivery.

For the owner: B0-DORIS-002 now compares with literal expected rows instead of
the seed's hand-written reference query; design.md gives the reason.

Open: harness cleanup of the Doris tier (stored-golden assertions, fixture,
final line, `docs/agents/doris-container.md`). Then Presto T2 on the manifest
(#61, B0-PRESTO-003), Druid (B0-DRUID-001 to 004) and T1 (B0-PRESTO-002). T3 on
real clusters is run by a person.

Learned: OpenSpec treats a scenario title as its identity, so a MODIFIED
requirement must keep existing titles; only the steps can change. Every Doris
parameter in the matrix is text, including numbers and booleans, which the SQL
casts; the number and bool manifest types are still unused.

## 2026-11-23: delivery blocked by review packet size, scope split

`scripts/dev deliver` passed local CI, Presto Docker and Doris Docker (39 cases)
at 14deb81 and then stopped: `Review context exceeds 500 KB; split scope before
review`. The packet embeds every changed file at base and head; `Cargo.lock`
(changed only by the new `insta` dev-dependency edge) is about 157 KB of it, and
the encoded packet is about 525 KB even without extra review context.

Split: the `insta` dev-dependency and the golden test travel first in
`add-execution-case-goldens` with the thirteen existing cases. After it merges,
merge main into this branch; this change then no longer touches `Cargo.lock`
and only grows the snapshot. No review has run on this branch yet.
