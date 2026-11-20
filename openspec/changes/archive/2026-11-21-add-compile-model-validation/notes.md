# Session notes

## 2026-11-20

User authorized next-PR design and development. Started from main after #26 merged.
Selected compile-time model validation as an independently testable boundary.
Full classified errors require a separate raw-query/unsupported contract and remain
in the seed; no existing scenario is partially transferred or claimed complete.

Design and implementation complete; final review pending. Scope is compile facade/tests,
this change and model-validation main spec. No harness, adapter or vendor edits.

Public regression failed before implementation: an invalid unused dimension still
produced SQL. It now returns exactly validate_model's complete report for both
structural and SQL faults, with valid/invalid query and security combinations.
Compile tests pass. Old join-query fixtures were given required primary keys;
invalid missing-target/duplicate-join cases now assert model errors. Existing
golden snapshots remain unchanged. The additive CompileError::Model variant
requires exhaustive-match callers to handle model issues. No lockfile changes.

Working-tree CI passed 15 steps, 0 skipped. Strict OpenSpec validation passed
7 items. Scope is below 400 lines and no harness/vendor/snapshot edits occurred.
Implementation checkpoint ccbb153 was pushed to PR #27. The requirement is
synced into the main spec and this folder is archived before final review, as
repository workflow requires. Final cross-vendor review and exact-head CI evidence
remain delivery work; owner approval and merge follow. Evidence is stored on the
PR without changing the reviewed commit. T1/T2 not run.
