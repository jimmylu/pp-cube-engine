# Tasks

Each task is bounded to two hours. No code is written before this proposal has merged.

## 1. Types and loading (first implementation pull request)
- [x] 1.1 Add `serde`, `serde_json`, `serde_norway` 0.9 and the JSON parser `serde_json_lenient` 0.2 to `pp-cube-model`; verify the `cargo deny` step of `scripts/check.sh ci` passes. If it reports an advisory, stop and open a `decision` issue.
- [x] 1.2 Define the contract types: model, cube, dimension, measure, measure filter, and the enums for dimension type and measure type; add `load_yaml` and `load_json`; verify with a test that the types are `Send + Sync` and survive a `serde_json` round trip.
- [x] 1.3 Give the load error its position; verify that an unknown field and an unknown type value each fail to load at the reported position, that a measure other than `count` without `sql` fails to load, and that a `meta` number which cannot be kept exactly fails to load from both formats.
- [x] 1.4 Add the test for `// scenario: B0-MODEL-001`: equivalent YAML and JSON fixtures with every dimension type, measure type and optional field load equal, with `meta` unchanged; verify with `cargo test -p pp-cube-model`.

## 2. References (second implementation pull request)
- [x] 2.1 Add the join, relationship and segment types; implement the reference scanner for the three forms; verify each form and each malformed brace case in unit tests.
- [x] 2.2 Implement resolution against a loaded model; verify that an unknown cube or member returns an error naming it.
- [x] 2.3 Add the test for `// scenario: B0-MODEL-004`: a join, a segment and expressions using each form resolve to their declared targets; verify with `cargo test -p pp-cube-model`.

## 3. Close
CI and the review by another vendor run through `scripts/dev deliver`; their evidence is on the pull request, bound to its commit, not in this file.
- [x] 3.3 Archive in the last implementation pull request; verify the traceability step reports 3 scenarios specified and 3 named by tests.
