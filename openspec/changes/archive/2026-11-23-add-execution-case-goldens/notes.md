# Notes

## 2026-11-23: split out of add-doris-b0-acceptance

Delivery of `add-doris-b0-acceptance` (#64) stopped at the controller's 500 KB
review packet limit: changed files are embedded at base and head, and
`Cargo.lock` alone is about 157 KB. The controller asks for a scope split, so
the `insta` dev-dependency and the golden test land here first, with the
thirteen cases already on main. Implemented with Claude Code (Anthropic).

Done: B0-EXEC-003 and its test; one snapshot with thirteen blocks; no generated
SQL, manifest content or existing golden changed.

Local results on the working tree that became the implementation commit:
- `check ci: PASSED — 15 steps, 0 skipped, 167s @ 0af8662+dirty`
- `check engines-container: PASSED — Presto 0.293, aggregate, text binding and Tokyo day goldens, 3 cases; not full T2 acceptance`
- `check doris-container: PASSED — Doris 4.1.3, 19 stored-golden assertions and 13 live-compiled manifest cases; not full T2 acceptance`
Exact-head evidence is posted on the pull request by delivery.

Open: after this merges, merge main into `change/add-doris-b0-acceptance`; its
golden test then also names B0-DORIS-001 and the snapshot grows to thirty-nine
blocks. B0-DORIS-001 and B0-DORIS-002 stay proposed in the seed until then.

Learned: a pull request that changes `Cargo.lock` has roughly 300 KB of review
packet left for everything else, including the fixed documents; plan dependency
edges as their own small change.
