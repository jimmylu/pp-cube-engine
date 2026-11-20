# Session notes

## 2026-11-20: planning after security filters

Prepared by Codex / OpenAI GPT-6. User requested next-PR planning and design.

Done:
- Confirmed PR #24 merged and its final different-vendor review had no findings; refreshed main and created change/add-model-validation from it.
- Compared structural validation, complete result metadata and another dialect. Recommended accumulated structural diagnostics as the next independently testable slice.
- Read model loading, types, reference resolution, metadata handling, nearby tests and the implemented model contract.
- Prepared proposal, design, delta and bounded tasks. Moved only Structured complete validation and B0-VALID-001 out of the seed; SQL/expression requirements stay there.
- Public interface proposal: validate_yaml, validate_json and validate_model return located ValidationIssue lists; existing loaders and compile remain compatible.

Open:
- Review this concrete plan, then request apply to begin implementation. All implementation tasks remain unchecked.
- Publish this planning slice as Draft and keep implementation/archive in the same PR.
- No product implementation, main-spec update or archive was performed. Different-vendor review and T1/T2 execution are not run in this planning session.

Learned:
- Required-field and enum failures are currently first-error deserialization failures; accumulating issues requires inspection before strict model decoding.
- Existing reference resolution can be reused, but independently parseable references need full traversal rather than short-circuiting.
- Metadata validation must retain the existing exact-number behavior; raw-tree conversion must not silently round values.
- Previous delivery review omitted workflow context after archive. Future review records must make the repository's pre-review archive rule explicit; do not change the harness in this PR.

Verification:
- Strict OpenSpec validation: six items passed, none failed; all four required planning artifacts exist.
- Fast check passed: four steps, none skipped (planning working tree). No source code, snapshots, dependencies, vendor or harness files changed.
- Exact-head CI evidence will be posted on the Draft PR after checkpoint. Planning checks do not prove the proposed validation behavior.

## 2026-11-20: implementation

User authorized implementation. Implemented by Codex / OpenAI GPT-6.
- Added document and typed-model validation with sorted JSON Pointer diagnostics, structural field rules and semantic source/name/reference/join checks. Existing loaders and compile signatures are unchanged.
- Eight public-interface tests name B0-VALID-001. Initial interface test failed on missing exports; a later failing test exposed references hidden by a malformed token, now recovered at brace boundaries using the shared reference parser.
- YAML/JSON diagnostics agree; strict JSON and exact metadata-number behavior are tested. Existing fixture, loader and compile regressions passed through CI.
- Working-tree CI passed 15 steps, none skipped. Exact-head evidence and cross-vendor review belong on PR #25 after checkpoint.
- Scope review: implementation, diagnostics tests and planning exceed 400 lines, but form one independently testable structural-validation behavior; no snapshot, vendor, lockfile, harness or adapter changes are needed.
- Completeness/correctness/coherence verification: B0-VALID-001 has assertions for its complete matrix; SQL constraints remain proposed in the seed. T1/T2 not run.
- Repository policy in docs/agents/workflow.md explicitly requires archive before final review in the same Draft PR. Sync the implemented structural requirement and archive now; owner approval and merge follow later.
- Open: checkpoint, exact-head CI, different-vendor review and Ready. Owner merges.
