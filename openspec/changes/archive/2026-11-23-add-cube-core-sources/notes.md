# Notes

## 2026-11-23: investigation, design and implementation

The owner asked to continue with R5, semantic guarantees, after #77 merged.
Product half of the Cube Core comparison. Implemented with Claude Code (Anthropic).

Established by hand before designing (details in design.md):
- The pin is the commit of tag `v1.8.28`; `cubejs/cube:v1.8.28` is the same Cube.
- Cube Core in development mode returns SQL from `/cubejs-api/v1/sql` with no
  database connection, and reads the conformance model file unchanged.
- All forty Presto cases: Cube Core's SQL and parameters equal the kernel's,
  except `LIMIT 10000` appended to a query without a limit.
- With the drafted harness tier, run locally: `check conformance: PASSED — Cube
  Core 1.8.28, 40 cases: SQL and parameters equal the kernel's but for Cube's
  default row limit, and results equal on Presto 0.293 (B0-PRESTO-002)`. That
  tier is not in this pull request; this is development evidence, not B0-PRESTO-002.

Done: `--sources` manifest (B0-EXEC-005); the comparison recorded in
`docs/guide.md`; Dockerfile base images pinned by digest.

Evidence: `check ci: PASSED — 15 steps, 0 skipped, 411s @ 2bb2446+dirty` on the
working tree that became the implementation commit; `scripts/check.sh image`
passed locally with the pinned bases. The plain Presto manifest is byte-identical
to main's. No golden changed. Remote checks are recorded on the pull request.

Open: the harness tier and its `cube-core` job; then transfer B0-PRESTO-002.
Not determined: whether Cube Core 1.8.28's default planner is the Rust planner
or the JavaScript one. #22, #23 and #30 are the next product change.

Learned: Cube's `/readyz` checks database connections, so readiness for this
use is a successful `/cubejs-api/v1/meta`. The Cube image is about 1 GB.
