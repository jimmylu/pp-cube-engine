# Notes

## 2026-11-23: proposal and implementation

Owner agreed in conversation to start this change once the harness integration
(#62, closing #59) merged. Implemented with Claude Code (Anthropic).

Done: the temporal execution tier requirement and B0-DORIS-010 moved from the
seed Doris delta into this change, reworded to the implemented behavior. The
conformance Doris cases name B0-DORIS-010. The transitional opt-in test and
`doris_time_engine.py` are deleted. No SQL, golden, manifest or dependency
changed.

Local results on the working tree that became the implementation commit:
- `check doris-container: PASSED — Doris 4.1.3, 19 stored-golden assertions and 13 live-compiled manifest cases; not full T2 acceptance`
- `check engines-container: PASSED — Presto 0.293, aggregate, text binding and Tokyo day goldens, 3 cases; not full T2 acceptance`
- `check ci: PASSED — 15 steps, 0 skipped, 196s @ a033f49+dirty`
Exact-head evidence is posted on the pull request by delivery, not committed.

Open: B0-DORIS-001 and B0-DORIS-002 stay proposed in the seed; the Doris matrix
audit, missing goldens and moving the 19 stored-golden assertions into manifest
cases are the next product change. `docs/agents/doris-container.md` still lists
this transfer as open; that sentence is a harness path and is left for the next
harness pull request. Presto tier migration is issue #61.

Learned: the traceability check reads `// scenario:` comments in any Rust file
under `crates/`, so an execution-tier scenario can be named on its case list
without an ignored Rust test.
