# Tasks

Each tracked task is bounded to two hours. Tests name B0-RESULT-002 and exercise
behavior; no new successful SQL snapshots are authorized.

## 1. Typed unsupported failures

- [x] 1.1 Add a failing public same-source multi-fact test, then add adapter/facade Unsupported variants and typed join rejection. Verify targeted adapter and compile tests, including existing direct/indirect multi-owner requests; document new enum variants in public API comments.
- [x] 1.2 Add overlapping-fault tests and preserve Model-first, unknown-member, security-filter and different-data-source Query precedence. Verify dependency-only multi-owner restrictions and unchanged ambiguous/disconnected join classifications with targeted tests.

## 2. Safe messages and internal mapping

- [x] 2.1 Audit locally authored and upstream-origin errors, implement safe public mapping, and test physical-source/SQL markers do not escape via Display or public model issues. Verify B0-RESULT-002 covers Model, Query and Unsupported through compile.
- [x] 2.2 Test a non-user planner failure through the actual adapter conversion and facade mapping seams; verify Internal and generic public Display while retaining operator diagnostics. Add no public injection switch; document the Display/Debug boundary beside the API.

## 3. Integration verification

- [x] 3.1 Run adapter/compile tests, strict OpenSpec validation and fast/ci; verify the full existing success snapshots remain unchanged and no vendor/harness/dependency changes occurred. Review scope size and update notes with exact commands/results and open work.

## Workflow follow-up

- Sync/archive the implemented Classified errors requirement before final review in this same Draft PR; main specs must retain Complete compile result.
- Checkpoint/push verified increments. Publish exact-head CI evidence and obtain cross-vendor review plus remote t0 before Ready. Evidence lives in PR comments/artifacts, outside tracked task progress.
- Owner performs final approval and merge. Do not archive or mark Ready while implementation tasks remain open.
