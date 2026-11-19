# Tasks

Each task is bounded to two hours. No code is written before this proposal has merged.

## 1. Types and loading (first implementation pull request)
- [ ] 1.1 Add `serde`, `serde_json` and `serde_yaml` 0.9 to `pp-cube-model`; verify the `cargo deny` step of `scripts/check.sh ci` passes. If it rejects the YAML parser, stop and open a `decision` issue.
- [ ] 1.2 Define the contract types: model, cube, dimension, measure, measure filter, join, segment, and the enums for dimension type, measure type and relationship; verify with a test that they are `Send + Sync` and survive a `serde_json` round trip.
- [ ] 1.3 Implement `load_yaml` and `load_json` with a load error that carries the position; verify that an unknown field and an unknown type value each fail to load.
- [ ] 1.4 Add the test for `// scenario: B0-MODEL-001`: equivalent YAML and JSON fixtures with every dimension type, measure type and optional field load equal, with `meta` unchanged; verify with `cargo test -p pp-cube-model`.

## 2. References (second implementation pull request)
- [ ] 2.1 Implement the reference scanner for the three forms; verify each form and each malformed brace case in unit tests.
- [ ] 2.2 Implement resolution against a loaded model; verify that an unknown cube or member returns an error naming it.
- [ ] 2.3 Add the test for `// scenario: B0-MODEL-004`: a join, a segment and expressions using each form resolve to their declared targets; verify with `cargo test -p pp-cube-model`.

## 3. Close
- [ ] 3.1 Run `scripts/check.sh ci` on each pushed commit; record the summary lines in notes.md.
- [ ] 3.2 Obtain a review from a vendor other than the implementer's; record the report and resolve findings.
- [ ] 3.3 Archive in the second implementation pull request; verify the traceability step reports 2 scenarios specified and 2 named by tests.
