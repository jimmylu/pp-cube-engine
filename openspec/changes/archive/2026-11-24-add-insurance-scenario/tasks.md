# Tasks

## 0. Route

- [x] 0.1 Review main against going live with the owner's deployment facts; evaluate the WebAssembly validator; record both in review.md. Verified: `pp-cube-model` builds for `wasm32-unknown-unknown` at `e94bc19`.
- [x] 0.2 Owner confirms or reorders the route and answers the decisions in review.md. Answered 2026-11-24; review.md revised.

## 1. Scenario

- [x] 1.1 Write the property-insurance model inside the contract; verify it validates.
- [x] 1.2 Write the business queries; compile each for Presto and Doris; record the support matrix in design.md.
- [x] 1.3 Put the queries to Cube Core 1.8.28; record agreement and differences.

## 2. Evidence

- [x] 2.1 Fixtures for both dialects and execution cases for the queries that compile; test B1-EXEC-001; verify existing cases' SQL is unchanged.
- [x] 2.2 Run `conformance` and `doris-container` locally; record the lines in notes.md.
- [x] 2.3 File an issue or name a change for each refusal the domain needs; update `docs/guide.md`.
- [x] 2.4 Run `scripts/check.sh ci`; record the last line in notes.md.

## Workflow follow-up

- Sync and archive the delta before final review, in the same Draft PR.
