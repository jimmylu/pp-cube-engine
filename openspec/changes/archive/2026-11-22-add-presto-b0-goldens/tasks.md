# Tasks

Each task is bounded to two hours. All implementation and verification tasks are complete.

## 1. Matrix coverage

- [x] 1.1 Synchronize main after the owner merges PR #34; verify its shared replay helper and accepted worker fix are present without duplicating the prior PR's diff.
- [x] 1.2 Mark the existing golden cases listed in design.md with B0-PRESTO-004 through B0-PRESTO-016 in table order, preserving current feature ids; verify all twenty operators and every other matrix member have executable snapshot coverage, then run aggregation, filters and joins integration suites.
- [x] 1.3 Add the five non-UTC granularity inputs in one loop test in time.rs with B0-PRESTO-001 and B0-DET-001; review their five new SQL/parameter snapshots with cargo insta review, then run the time suite with INSTA_UPDATE=no. Verify existing snapshots are unchanged and update the coverage table with actual new case names.

## 2. Integration verification

- [x] 2.1 Run all five golden suites with INSTA_UPDATE=no, then repeat with RUST_TEST_THREADS=1; verify complete-result process comparisons pass and record outcomes in notes.md.
- [x] 2.2 Run strict OpenSpec validation, fast and ci; inspect the diff for allowed paths, unchanged existing snapshots and no pending snapshots. Record the results and keep execution-tier requirements in the seed.

## Workflow follow-up

- The user requested implementation on 2026-11-22; continue delivery in the same Draft PR.
- Sync/archive only the implemented Presto B0 compilation requirement before final review; preserve its id and the seed's T1/T2 requirements.
- Checkpoint/push and deliver through exact-head CI, omp/Opus review and remote t0 before Ready. Owner approves and merges.
