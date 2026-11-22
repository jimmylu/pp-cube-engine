# Tasks

Each task is bounded to two hours. Planning is complete; implementation has not started.

## 1. Matrix coverage

- [ ] 1.1 Synchronize main after the owner merges PR #34; verify its shared replay helper and accepted worker fix are present without duplicating the prior PR's diff.
- [ ] 1.2 Mark the existing golden cases listed in design.md with B0-PRESTO-001, preserving current ids; verify all twenty operators and every other matrix member have executable snapshot coverage, then run aggregation, filters and joins integration suites.
- [ ] 1.3 Add the five non-UTC granularity inputs in time.rs with B0-PRESTO-001 and B0-DET-001; review their five new SQL/parameter snapshots with cargo insta review, then run the time suite with INSTA_UPDATE=no. Verify existing snapshots are unchanged and update the coverage table with actual new case names.

## 2. Integration verification

- [ ] 2.1 Run all five golden suites with INSTA_UPDATE=no, then repeat with RUST_TEST_THREADS=1; verify complete-result process comparisons pass and record outcomes in notes.md.
- [ ] 2.2 Run strict OpenSpec validation, fast and ci; inspect the diff for allowed paths, unchanged existing snapshots and no pending snapshots. Record the results and keep execution-tier requirements in the seed.

## Workflow follow-up

- Continue planning and implementation in the same Draft PR; stop after planning until the user requests implementation.
- Sync/archive only the implemented Presto B0 compilation requirement before final review; preserve its id and the seed's T1/T2 requirements.
- Checkpoint/push and deliver through exact-head CI, omp/Opus review and remote t0 before Ready. Owner approves and merges.
