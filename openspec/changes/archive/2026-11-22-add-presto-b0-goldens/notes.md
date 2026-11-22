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


## 2026-11-22: implementation and verification

The user requested implementation. Synchronized main after #34 merged as 29347eade530c357148ae4e56a41b3bc3200bc6d, using a normal merge after fast passed; no history rewrite. Confirmed the shared worker pins --test-threads=1 and puts its record on a new line.

Added B0-PRESTO-001 directly to the thirteen existing golden tests identified by the coverage table, retaining their other ids. Added five independent Asia/Tokyo granularity tests, each with B0-PRESTO-001 and B0-DET-001, using October closed ranges. Reviewed all five new snapshots with cargo insta review: matching date_trunc unit and alias, Tokyo conversion, two placeholders and ordered UTC bounds 2026-09-30T15:00:00.000Z / 2026-10-31T14:59:59.999Z. The initial run failed only because these snapshots were new; accepting reviewed snapshots made it pass. Existing snapshot bytes are unchanged; no pending snapshots remain.

Aggregation, filters and joins passed. All five golden suites passed with INSTA_UPDATE=no, then again with RUST_TEST_THREADS=1. Every new successful input runs through the existing four-process full-result replay. All twenty filter operators and every remaining matrix member are represented in executable golden inputs.

Working-tree checks (not committed-head acceptance):
- Both active changes passed strict OpenSpec validation.
- check fast: PASSED — 4 steps, 0 skipped, 62s (synchronized main plus uncommitted implementation)
- check ci: PASSED — 15 steps, 0 skipped, 99s (synchronized main plus uncommitted implementation)

OpenSpec verification: completeness 5/5 tasks; correctness covers the complete B0-PRESTO-001 matrix; coherence follows the approved reuse/new-input design. No critical findings or warnings. Scope remains tests and specs only, below 400 authored lines at review. Production code, templates, dependencies, vendor and harness are unchanged.

Sync/archive only the Presto B0 compilation requirement in this PR, as required by the lifecycle. T1/T2 acceptance remains in the seed; T1/T2 were not run. Exact-head CI, omp / Anthropic Claude Opus 5.5 review and remote t0 are pending after checkpoint; their final evidence belongs in PR comments. Owner approves and merges. No known implementation blocker.

Delivery follow-up: the first exact-head CI passed Rust tests but typos rejected a hexadecimal substring in the historical dirty-tree check identity above. Reworded those preparation records; no code or check configuration changed. Final-head acceptance remains pending in PR comments.


## Review follow-up: complete scenario coverage

Opus identified that the seed policy requires each test claiming a matrix scenario to exercise its whole member list. The initial collective-tag design violated that policy. Split B0-PRESTO-001 as the seed explicitly allows: retain 001 for all five non-UTC granularities in one loop test; allocate 004 through 016 to the thirteen existing matrix tests. Keep 002/003 reserved in the seed for T1/T2. The compilation requirement text and overall feature coverage are unchanged; five snapshots retain their SQL and parameters. Earlier notes describing collective tags and verbatim scenario transfer are superseded by this correction.

Provide the seed and implemented compile-b0 specs as supplemental review context to answer the separate context question. No runtime or harness scope is added. Final-head verification and review remain pending.


Second review repair: Opus resolved the scenario-splitting finding and confirmed all twenty operators from query.rs. It then identified stale expression headers in the five new Tokyo snapshots. Regenerated those files from the loop and reviewed them with cargo insta review. Their SQL/parameter bodies are byte-identical; only expression metadata changes. The snapshot-generation run used INSTA_FORCE_PASS solely to collect every pending loop snapshot; acceptance uses ordinary INSTA_UPDATE=no tests and CI, without that override. Both default and single-thread runs of the complete loop pass after review. No pre-existing main snapshot is changed. This completes the second substantive review repair; if further defects remain, record the bounded stop rather than continuing repairs indefinitely.
