# Tasks

Each task is bounded to two hours.

## 1. Process comparison
- [x] 1.1 Add shared worker/wrapper with full-result comparison and comparison-sensitivity tests naming B0-DET-001; verify worker transport and failure handling through integration tests.
- [x] 1.2 Route all five existing B0 golden suites through the wrapper; verify every successful golden input runs in four fresh processes, including inline and loop-generated snapshots, without changing goldens.

## 2. Verification
- [x] 2.1 Run compile tests, strict specs and CI; confirm no production/dependency/vendor/harness or snapshot changes and record scope/results in notes.md.

## Workflow follow-up
- Sync/archive only implemented requirements before final review in the same PR.
- Checkpoint/push; publish exact-head CI, cross-vendor review and remote t0 before Ready. Owner reviews and merges.
