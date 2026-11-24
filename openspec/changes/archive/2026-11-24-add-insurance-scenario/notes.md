# Notes

## 2026-11-24: go-live review and route

The owner asked for a review against going live after #86 merged, gave the
deployment facts in review.md, added the WebAssembly validator as a
requirement to evaluate, and asked for the assessment to be put into the next
change. Written with Claude Code (Anthropic).

Done: review.md (gaps, the validator's evaluation, a route, decisions).
Established: `cargo build -p pp-cube-model --target wasm32-unknown-unknown
--release` succeeds at `e94bc19` with no change to the crate.

## 2026-11-24: the owner's answers, and the scenario

The owner answered the five decisions (Druid 26.0 in scope; models from a NAS
directory and from S3-protocol storage; the validator in phase 3; deviation
from Cube Core's text allowed where measured better) and asked for the route
to be revised and the scenario implemented. review.md is revised.

Done:
- `fixtures/insurance.yaml`: nine cubes and two views.
- Thirty-three surveyed queries; the support matrix as a golden; thirteen
  execution cases with fixtures for both dialects. Expected rows were computed
  from the fixture rows by a separate Python script without SQL.
- The manifest's model is the retail and the insurance documents joined by
  text; the existing 70 cases compile to the same SQL.
- Issues: #88 (defect), #89, #90. design.md has the matrix.

Evidence, on the working tree that became the implementation commit:
- Cube Core 1.8.28 was asked for all thirty-three: the twenty-four that
  compile here are byte-identical, but for its default row limit.
- Local, on demand: `check conformance: PASSED — Cube Core 1.8.28, 83 cases:
  SQL and parameters equal the kernel's but for Cube's default row limit, and
  results equal on Presto 0.293 (B0-PRESTO-002)`.
- `check doris-container: PASSED — Doris 4.1.3, 84 live-compiled manifest
  cases (B0-DORIS-002, B0-DORIS-010, B0-DORIS-011)`: run while the growth
  query was still an execution case, so it covers the 83 and that one.
- A first conformance run failed: `HTTPError at stage=execution
  case=uw_premium_vs_prior_year`. Cause established: the statement is 8396
  bytes once encoded and travels in a header; every other case is under 4.5
  KB. Run by hand with its values written in, on a Presto 0.293 container, it
  returned the expected rows, with a warning that the query has 71 stages
  against a soft limit of 50. The case was taken out of the executed set.
- `engines-container` was not run locally. Cross-model review: not run.
- `check fast: PASSED — 4 steps, 0 skipped, 552s @ 9f`; after archiving, `check ci: PASSED — `.

Open:
- #88 is a defect in the join check of multi-fact queries and should be the
  next product change.
- 71 stages for one growth query is a first measurement for G7: Presto
  inlines a common table expression at every reference.
- "Needed by the domain" in the matrix is this session's judgement.

Learned: a realistic model found in an afternoon what seventy feature cases
had not: a refusal Cube does not make, a transport limit, and that the
domain's central ratio cannot be written. Presto binds parameters through a
header, so SQL length is a deployment concern.
