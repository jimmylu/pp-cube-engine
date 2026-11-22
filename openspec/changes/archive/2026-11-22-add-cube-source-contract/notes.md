# Session notes

## 2026-11-22: design and implementation
User explicitly authorized next-PR design and implementation together. PR #37 is merged; branch starts from its main tip. Selected Cube source contract over a new dialect or bundle API to close a bounded remaining B0 requirement. All design choices are routine; no owner-only decision is open. The full Cube fields requirement moves from the seed under its transfer policy; immutable bundles stay there. Implementer: Codex / OpenAI GPT-6. Planned reviewer: omp / Anthropic Claude Opus 5.5.

Planning artifacts complete; implementation and verification follow in this same PR. No production output change is expected. T1/T2 not run.

Verification complete: both focused model tests pass; full model tests are included in passing workspace CI. Strict validation passes for this change and the remaining seed. Fast passed all four steps with no skips; CI passed all fifteen steps with no skips on the working tree. These are preparation results, not committed-head acceptance. No snapshot, production, dependency, vendor or harness edits.

OpenSpec verification: all three tasks complete; B0-MODEL-002 is exercised by one test covering all four failure cases in both encodings. B0-MODEL-006 is exercised by one test covering all accepted fields with each source, field preservation, both round trips and unknown-field rejection. No critical findings or warnings. Each matrix scenario has complete coverage in a single test, following the seed rule.

Sync/archive the implemented requirement into existing contract-model specs before final review. Immutable bundles remain proposed in the seed. Final exact-head CI, omp/Opus review and remote t0 are pending publication and will be recorded in PR comments. Owner approves and merges. No known blocker; T1/T2 not run.
