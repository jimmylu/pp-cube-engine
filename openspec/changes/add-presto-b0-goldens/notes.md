# Session notes

## 2026-11-22: proposal preparation

Requested scope: plan the next PR after completing #34 through Ready. Prepared with Codex / OpenAI GPT-6. No implementation or archive is authorized by this planning session.

At session start #34 is Ready, OPEN and unmerged at e87e22ea3ed904518a220c8ab1e16b4271f815dc. Main is the merged harness PR #35 tip. This planning branch starts from main, keeping #34 intact. Implementation depends on synchronizing main after its owner merge.

Selected add-presto-b0-goldens over introducing another dialect or execution infrastructure. Existing B0 aggregation/filter/join/time goldens already cover most of Presto's matrix; five granularities outside UTC are the concrete missing acceptance inputs. The design contains a test-by-test matrix and a bounded plan for five new snapshots. Production SQL and existing snapshots are expected to stay unchanged.

Transferred only Presto B0 compilation / B0-PRESTO-001 verbatim from the seed, following its transfer policy. Presto T1/T2 acceptance remains there. No other seed requirements, production files or harness files were changed. Issues labelled after-baseline remain deferred, including #36.

Proposal, spec delta, design and implementation tasks are written. All implementation tasks remain unchecked. No owner-only scope decision is open. Validation and publication evidence will be recorded below; final acceptance belongs in SHA-bound PR evidence.

Validation: both add-presto-b0-goldens and the remaining seed pass strict OpenSpec validation; all four planning artifacts are complete. git diff --check passed. Working-tree check:
`check fast: PASSED — 4 steps, 0 skipped, 16s @ 68fe053+dirty`.
This is preparation evidence, not evidence for a committed head. CI and cross-vendor review have not been run for this planning slice; T1/T2 are not run. Next: publish the proposal as Draft, then wait for a user request to implement and the #34 merge prerequisite.
