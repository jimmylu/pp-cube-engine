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
