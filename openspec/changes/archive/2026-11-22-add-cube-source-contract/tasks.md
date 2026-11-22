# Tasks
Each task is bounded to two hours.

## 1. Model entry coverage
- [x] 1.1 Add B0-MODEL-002 test for all four source-selection failures in YAML/JSON; verify exact issue paths/rules with cargo test -p pp-cube-model --test cube_sources.
- [x] 1.2 Add B0-MODEL-006 test covering all accepted cube fields, both sources and an unknown field; verify preserved fields and both format round trips with the same test command.

## 2. Integration
- [x] 2.1 Run full model tests, strict specs, fast and ci; confirm the diff contains only scoped tests/specs and record evidence in notes.md.

## Workflow follow-up
- Sync/archive implemented requirements, retaining immutable bundles in the seed.
- Checkpoint and deliver through exact-head CI, omp/Opus review and t0; owner approves and merges.
