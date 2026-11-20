# Tasks

Implementation authorized. Each implementation task is bounded to two hours.

## 1. Parser boundary

- [x] 1.1 Add the pinned parser dependency and a failing public validation test naming B0-VALID-002, then wire a private SQL-checking module into all three validation entry points. Verify valid baseline SQL, parsing failures, located diagnostics and crate/toolchain compatibility; record dependency licence and lockfile scope.
- [x] 1.2 Add B0-VALID-002 tests for multiple statements, aggregation, DISTINCT, windows, joins, derived/nested queries, sets and CTEs. Implement the AST allowlist and recursive checks; verify each case reports at its SQL field and valid projection/WHERE cases pass.

## 2. Expressions and source preservation

- [x] 2.1 Test reference substitution through public validation with B0-VALID-004. Implement parsing-only sentinels based on shared reference spans; verify quoting, own columns, declared cross-cube members and collision handling, with original model text unchanged.
- [x] 2.2 Add failing B0-VALID-004 tests for nested and indirect aggregate use in dimensions/segments. Implement expression/dependency traversal with cycle detection; verify ordinary measures, filtered measures and number formulas remain valid, and document built-in aggregate classification sources.
- [x] 2.3 Add B0-VALID-004 tests for raw cross-cube columns, then enforce provenance checks. Verify declared-member access remains valid, physical column names are not leaked in messages, and join conditions/row filters retain row-level semantics.

## 3. Combined acceptance

- [x] 3.1 Add B0-VALID-003 tests for one_to_many, undeclared references, time_shift, rolling_window and pre-aggregations. Verify located rejection for each alone and mixed with SQL faults, across YAML/JSON/typed entry points where representable; preserve structural diagnostic regressions.
- [x] 3.2 Run model/compile tests and strict OpenSpec validation. Verify meaningful coverage of all three ids, unchanged existing golden SQL, metadata and source text, and no vendor/harness edits; record scope review and remaining limitations in notes.md.
- [x] 3.3 Run fast before checkpoint and CI before final review. Verify exact summary lines and dependency review, apply risk:upstream if Cargo.lock changes, and publish SHA-bound evidence outside Git.

## Workflow follow-up

- Implementation authorized and completed in this Draft PR.
- Archive the implemented requirement into the existing main spec before final review, per repository workflow. Other seed capabilities remain proposed.
- Obtain different-vendor review and successful remote t0 before Ready. Owner approves the final head and merges.
