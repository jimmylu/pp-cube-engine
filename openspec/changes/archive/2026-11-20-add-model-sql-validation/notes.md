# Session notes

## Current state: 2026-11-20

Implementation and all eight tasks are complete. The Baseline SQL and joins
requirement, including B0-VALID-002 through B0-VALID-004, is synced into
openspec/specs/model-validation/spec.md. This change is archived in
openspec/changes/archive/2026-11-20-add-model-sql-validation/ before final review,
as required by repository workflow. Remaining delivery work: obtain a clean
cross-vendor review and publish exact-head evidence, then request owner review.
The owner merges. PR: https://github.com/jimmylu/pp-cube-engine/pull/26

Implemented with Codex / OpenAI GPT-6. DeepSeek V4 Pro reviewed eee45b2 and
e4010c3 through omp; it reported documentation corrections, with no code findings.
Corrected the new syn version and reconciled the archived handoff state. Final
review/evidence are recorded outside Git to avoid invalidating their head SHA.

## Behavior and validation

The three existing public validation APIs now parse cube SQL and member
expressions without rewriting source text. Tests cover row-query restrictions,
parsing-only reference sentinels, nested/indirect aggregation, cross-cube physical
columns, unsupported model features, format parity and excessive nesting.
Member dependency traversal terminates on cycles; a separate cycle rejection
policy is not introduced. GenericDialect remains the parsing boundary. Opaque
UDF semantics require catalog knowledge; no function allowlist is introduced.

The initial malformed-SQL test and forbidden-query matrix failed before their
implementation and passed afterward. Model/compile tests, metadata/source
preservation and existing golden snapshots pass. Strict OpenSpec validation
passed both before and after archive. T1/T2 are not implemented and were not run.
Exact-head CI passed 15 steps with 0 skipped at 7d5b712 (36s), eee45b2 (39s)
and e4010c3. SHA-bound evidence and review reports live on the PR.

## Dependency and scope review

Pinned sqlparser =0.64.0 (Apache-2.0), with std/visitor. The default optional stack
extension introduced ar_archive_writer's Apache-2.0 WITH LLVM-exception licence,
which CI rejected. Disabled that optional feature instead of changing policy;
the parser's recursion counter remains enabled, covered by a nesting regression.
Final new dependencies: sqlparser, sqlparser_derive 0.7.0 and syn 3.0.10.
Existing package versions and upstream pin are unchanged. risk:upstream is applied.

Built-in aggregate classification was checked against official indexes:
- https://github.com/prestodb/presto/blob/master/presto-docs/src/main/sphinx/functions/aggregate.rst
- https://github.com/apache/doris-website/tree/master/docs/sql-manual/sql-functions/aggregate-functions
- https://druid.apache.org/docs/latest/querying/sql-aggregations/

Scope exceeds 400 lines including tests/lock entries. Parsing, reference provenance
and aggregate dependencies remain one independently testable validation boundary.
No vendor, harness, root configuration or existing SQL snapshots changed. One
Clippy type-complexity diagnostic was fixed using a private result alias; fast
passed afterward. No owner-only decision is open.
