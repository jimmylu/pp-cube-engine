# Session notes

## 2026-11-20: planning after structured validation

Prepared by Codex / OpenAI GPT-6. User requested next-PR planning and design.

Done:
- Confirmed PR #25 merged and started this branch from updated main.
- Selected remaining B0 model SQL/expression validation over complete result metadata or another dialect.
- Read the existing validator, reference/contract types, public parser design, manifests/lockfile and acceptance scenarios.
- Created proposal/design/tasks and transferred the complete Baseline SQL and joins requirement with B0-VALID-002 through B0-VALID-004 intact. The seed's now-empty model-validation file is removed.
- Checked sqlparser 0.64.0 primary documentation for GenericDialect, AST parsing and source preservation concerns. The parser is not already in the dependency graph.

Open:
- Review the concrete plan, then request apply. All eight implementation tasks remain unchecked.
- Publish planning as Draft; implementation and archive continue in that same PR.
- No product code, parser dependency, lockfile, main spec or snapshot changes in this planning session. Different-vendor review and T1/T2 execution not run.

Learned:
- The current public validator is explicitly structural, so SQL checks must extend its report semantics and documentation without changing signatures.
- Parsing must not rewrite source SQL, and Cube reference provenance must survive in a separate parsing representation.
- Syntax alone cannot prove opaque UDF semantics. No new function allowlist or catalog dependency is silently authorized; raise a decision if implementation needs one.
- The repository requires archive before final review in the same Draft PR. Preserve this policy context for reviewers after the folder moves.

Verification:
- Strict OpenSpec validation: seven items passed, none failed; all four required planning artifacts exist.
- Fast check passed: four steps, none skipped. Official versioned parser documentation confirms GenericDialect and Parser::parse_sql.
- Exact-head CI evidence will be posted on the Draft PR after checkpoint. These checks validate planning consistency, not unimplemented SQL validation behavior.

## Implementation: 2026-11-20

Implemented with Codex / OpenAI GPT-6. Public validation now parses SQL without
changing source text. Model tests pass across YAML, JSON and typed entry points.
The initial malformed-SQL test failed before implementation and passed afterward;
the row-query rejection matrix likewise exposed the missing construct checks.
One fast-check failure was a Clippy type-complexity diagnostic, fixed with a
private result type alias; the subsequent fast check passed (4 steps, 0 skipped).

Dependency review: sqlparser =0.64.0, Apache-2.0, visitor enabled; optional stack extension disabled because its transitive LLVM-exception licence
is outside the existing policy; parser recursion limit retained. New transitive packages are parser/visitor support. Existing package versions and upstream pin remain unchanged. Cargo.lock
requires risk:upstream. No root configuration, harness, vendor or snapshots edited.

Built-in aggregate classification was checked against official source indexes:
- https://github.com/prestodb/presto/blob/master/presto-docs/src/main/sphinx/functions/aggregate.rst
- https://github.com/apache/doris-website/tree/master/docs/sql-manual/sql-functions/aggregate-functions
- https://druid.apache.org/docs/latest/querying/sql-aggregations/
Opaque UDF semantics still require catalog knowledge; no function allowlist is
introduced. GenericDialect remains the parsing boundary. Cyclic member traversal
terminates; a separate cycle rejection policy is not introduced by this change.

Scope review: above 400 lines including tests/dependency lock entries. Kept together
because parsing, reference provenance and aggregate dependencies implement one
independently testable validation boundary. Existing compile golden output and
metadata regressions run through the repository check command.

Final archive occurs before final review in this same Draft PR, as required by
repository workflow. Exact-SHA CI/review evidence belongs in PR comments/artifacts.

CI initially rejected ar_archive_writer's Apache-2.0 WITH LLVM-exception licence,
introduced by sqlparser's optional recursive stack extension. Disabled that optional
feature in the crate manifest instead of changing harness policy. The parser's
own recursion counter remains enabled, with a public-interface excessive-nesting
regression. Final dependency additions are sqlparser, sqlparser_derive and syn 1.x;
existing versions are unchanged.

Final implementation checkpoint: 7d5b7122e81a4973fc525ca7c5d54f371675acfd, pushed to PR #26.
Exact-head check ci: PASSED — 15 steps, 0 skipped, 36s @ 7d5b712.
All eight tasks complete. Evidence posted outside Git. Final archive/review is
the remaining delivery step; the owner merges. T1/T2 not run.
