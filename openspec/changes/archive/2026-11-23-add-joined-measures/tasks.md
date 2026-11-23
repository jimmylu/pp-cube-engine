# Tasks

## 1. Establish

- [x] 1.1 Put thirteen join queries to Cube Core 1.8.28 under both of its planners; record what it emits in design.md.
- [x] 1.2 Trace the pinned planner's use of the speculative join lookup; record the finding for #30.

## 2. Joins

- [x] 2.1 Allow measures of a joined cube that stay flat, name the ones that do not, and refuse multi-stage SQL; test B0-JOIN-006 with goldens; verify `cargo test -p pp-cube-compile --test joins`.
- [x] 2.2 Join the cubes a condition reads before it; test B0-JOIN-007 and the unorderable case of B0-JOIN-005.
- [x] 2.3 Pin the speculative lookup behaviour with the B0-JOIN-008 regression in the adapter.
- [x] 2.4 Update the Doris boundary test (B0-DORIS-006) and the security test.

## 3. Evidence

- [x] 3.1 Add three execution cases with fixtures for both dialects; verify existing cases' SQL is unchanged and run `conformance`, `engines-container` and `doris-container` locally.
- [x] 3.2 Transfer B0-PRESTO-002 from the seed; update `docs/guide.md`; verify `openspec validate --all --strict` and traceability.
- [x] 3.3 Run `scripts/check.sh ci`; record the last line in notes.md.

## Workflow follow-up

- Sync and archive the deltas before final review, in the same Draft PR.
- Close #22, #23 and #30 when this merges.
- Decide the first version number now that the Cube Core comparison is in place (owner).
- Next unit of the route: B1 multi-fact queries, which also covers a root measure beside a one_to_one measure.
