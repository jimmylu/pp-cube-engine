# Notes

## 2026-11-24: fix and evidence

The owner asked for #88 after #87 merged. Implemented with Claude Code
(Anthropic).

Done: `Joins::check` accepts a fact without a join tree of its own when one
tree holds every cube the query reads; regression B1-MULTI-008; the scenario
matrix moves two queries to compiled, and both are execution cases (85 in all).

Evidence, on the working tree that became the implementation commit:
- Thirteen queries around the defect against Cube Core 1.8.28: design.md.
- Local, on demand, both on their first run: `check conformance: PASSED —
  Cube Core 1.8.28, 85 cases: SQL and parameters equal the kernel's but for
  Cube's default row limit, and results equal on Presto 0.293
  (B0-PRESTO-002)`; `check doris-container: PASSED — Doris 4.1.3, 85
  live-compiled manifest cases (B0-DORIS-002, B0-DORIS-010, B0-DORIS-011)`.
- Goldens: the matrix changes in exactly the two lines the delta names; the
  manifest goldens gain two blocks each and lose no line.
- `engines-container` was not run locally. Cross-model review: not run.
- CI_EVIDENCE

Open: #89, #90, #83 stay as the matrix records them. Next on the route: G1.

Learned: counted from payments, Cube's SQL gives a reserve only to claims
that were paid on. The two ways of asking for outstanding by line (from
claims, from payments) differ by the open claim without a payment; both are
pinned by a case.
