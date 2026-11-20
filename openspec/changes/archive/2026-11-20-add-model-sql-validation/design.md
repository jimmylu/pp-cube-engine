# Design

## Context

PR #25 is merged. `validation.rs` already accumulates shape, source, reference and join diagnostics through YAML/JSON/typed-model entry points. It explicitly does not inspect SQL semantics. The remaining seed requirement has three scenarios; transfer that whole requirement here without renumbering.

Read the model validator and contract types, existing model and compile fixtures, reference parser, Cargo manifests/lockfile and public engine design section 6.1. No SQL parser is currently in the root dependency graph. The public design already specifies parsing without rewriting.

## Goals / Non-Goals

Goals: enforce the remaining B0 model SQL restrictions through the existing report APIs, preserve source text and keep valid compilation output stable.

Non-goals: execute SQL, resolve physical catalogs, add dialect generation, change compile signatures, inspect arbitrary UDF implementations or alter query-dependent join rules.

## Decisions

### Sequence and alternatives

Finish model validation before expanding complete results or new dialects. This reuses one report boundary and closes a known validation gap.

Use an SQL AST, as the public design requires. Regex-only checking cannot reliably distinguish strings/comments from syntax or find nested subqueries. Reusing the planner would mix model validation with query-dependent planning. Keep parsing and checks in a private model-crate module.

### Dependency and parser boundary

Use `sqlparser` with its visitor support, pinned to a tested release in the crate manifest/lockfile. The current official documentation describes release 0.64.0 and separate statement/expression ASTs:
[crate documentation](https://docs.rs/sqlparser/0.64.0/sqlparser/),
[expression AST](https://docs.rs/sqlparser/0.64.0/sqlparser/ast/enum.Expr.html).

Start with GenericDialect for the baseline's SQL text. A syntax failure is a located validation issue; never fall back to string acceptance after parsing fails. Prove fixture compatibility before accepting the dependency choice. No toolchain, root patch or harness change belongs here. Cargo.lock changes require the repository's risk:upstream label and dependency review, without moving the upstream pin.

The parser supplies syntax, not catalog/type/function semantics. Maintain an explicit, source-reviewed table of built-in aggregate names for the supported B0 SQL surface, including case-insensitive and qualified spelling. Do not claim to determine what an arbitrary UDF does. If enforcing a requested function needs catalog information or a new function allowlist that changes the existing expression contract, stop and raise an owner decision rather than silently accepting or narrowing the requirement.

### Cube SQL allowlist

Accept exactly one query statement, one plain SELECT body, one physical table source with optional alias, projections and an optional row WHERE predicate. Permit whitespace, comments, quoted identifiers and a trailing terminator. Reject statement lists and DDL/DML, CTEs, DISTINCT, GROUP BY/HAVING, aggregation, windows, joins, comma-separated sources, derived tables, table functions, nested queries anywhere and set operations. Other query clauses are outside the row-level form and produce a located issue.

Walk the entire tree, including nested function arguments, predicates and projection expressions. A forbidden construct under a scalar wrapper is still forbidden. Treat parser variants not explicitly accepted as unsupported SQL constructs.

### Member expressions and references

For parsing only, replace Cube brace references with collision-free sentinel identifiers derived from existing located-reference spans. Retain an association to the original references, including own-cube columns and declared members. Never write transformed text back into the model. Structural validation continues to own malformed/unresolved references and unsupported fields/relationships.

Parse each dimension, measure, measure-filter, segment and join expression as one complete expression; reject leftover statements or tokens. Check aggregates outside measures, including dimension/segment expressions and row predicates. Preserve ordinary aggregate measure inputs, filtered measures and number formulas. A dimension that refers to an aggregate measure must not bypass the dimension aggregation restriction: follow member dependencies with cycle detection and visited sets.

Inspect compound identifiers and reference provenance together. Own-cube physical columns and declared member references are allowed; a raw reference to another cube's physical column is rejected even when a join exists. Preserve legitimate physical column qualification and quoting in the owning cube. Do not confuse a declared `{other.member}` with forbidden `other.column` access.

### Diagnostics and compatibility

Keep all three public validation signatures and `ValidationIssue` unchanged. Reuse JSON Pointer field paths and deterministic sorting. Add stable rule ids such as `sql-syntax`, `sql-construct`, `expression-aggregate` and `expression-column`. Messages identify the rule and object without echoing SQL or physical schema text.

Continue collecting independent SQL faults across fields/cubes alongside structural issues. Skip a field whose shape/reference fault already makes parsing meaningless; avoid duplicate cascades. A successful validation must not alter model text, metadata or compiled SQL.

## Validation

- B0-VALID-002: one case for each forbidden construct, including nested placements, plus valid projection/WHERE, comments, quoting and trailing terminator. Assert field paths/rules, not parser error strings.
- B0-VALID-003: one_to_many, undeclared references, time_shift, rolling_window and pre-aggregations still fail through existing structural checks; test them individually and mixed with SQL faults.
- B0-VALID-004: aggregates in dimensions/segments, nested and indirect aggregate references, forbidden cross-cube columns, and allowed own-column/declared-member forms.
- Run equivalent YAML/JSON and typed-model checks; assert original SQL text is unchanged and existing compile snapshots still pass.

## Risks / Trade-offs

Dialect syntax coverage and built-in function classification need focused fixture tests and pinned sources. Unknown function semantics cannot be proved without catalog metadata. No execution or broad SQL safety claim follows from these checks. Size is likely above 400 lines; retain one independently testable validation behavior and review scope before implementation.
