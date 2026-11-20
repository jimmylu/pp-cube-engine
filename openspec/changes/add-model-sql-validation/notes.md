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
