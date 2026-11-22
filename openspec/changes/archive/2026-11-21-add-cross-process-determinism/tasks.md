# Tasks

Each task is bounded to two hours.

## 1. Process comparison
- [x] 1.1 Add shared worker/wrapper with full-result comparison and comparison-sensitivity tests naming B0-DET-001; verify worker transport and failure handling through integration tests.
- [x] 1.2 Route all five existing B0 golden suites through the wrapper; verify every successful golden input runs in four fresh processes, including inline and loop-generated snapshots, without changing goldens.

## 2. Verification
- [x] 2.1 Run compile tests, strict specs and CI; confirm no production/dependency/vendor/harness or snapshot changes and record scope/results in notes.md.
- [x] 2.2 Attach B0-DET-001 directly to all 28 successful golden-suite tests that exercise process replay, including the mixed success/error time-order regression.

- [x] 2.3 Reproduce single-thread worker output failure, delimit records, pin worker test threads and rerun all five golden suites with RUST_TEST_THREADS=1.

## Workflow follow-up
- Sync/archive only implemented requirements before final review in the same PR.
- Checkpoint/push; publish exact-head CI, cross-vendor review and remote t0 before Ready. Owner reviews and merges.
- Previous session stopped after bounded review repairs; the owner authorized resumption after harness PR #35 merged. Synchronize main and rerun exact-head CI, Opus review and t0 before Ready.
