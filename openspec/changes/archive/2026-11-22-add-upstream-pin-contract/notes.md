# Session notes

## 2026-11-22: design and implementation
User authorized next-PR design and implementation together. Based on main after merged #38. Selected the remaining upstream identity contract before a new bundle API or dialect. No owner-only decision is open. Transfer the complete seed upstream-pin requirement under its transfer policy. Implementer: Codex / OpenAI GPT-6; reviewer planned: omp / Anthropic Claude Opus 5.5. Scope is existing test code and specs; vendor remains read-only and the pin stays unchanged. T1/T2 not run.


Verification complete: the focused repository test passed with --nocapture and emitted neither skip message. Full compile tests passed in workspace fast/CI. The test compares gitlink, checked-out upstream HEAD and the public compiled fingerprint, then verifies a different pin input changes the expected identity. Both active changes passed strict validation. Fast passed 4 steps with no skips; working-tree CI passed 15 steps with no skips. These are preparation checks, not committed-head acceptance.

OpenSpec verification: 2/2 tasks complete; one test covers the full B0-UPSTREAM-001 scenario. No critical findings or warnings. Test-only changes in fingerprint.rs; no production code, pin, vendor, dependencies, snapshots or harness changes. Existing source-archive/missing-Git skip semantics are retained, but were not used for acceptance. The mandatory vendor check also passed.

Sync/archive the complete tested requirement in this PR. Final exact-head CI, omp/Opus review and remote t0 remain pending publication; acceptance will be recorded in SHA-bound PR comments. Owner approves and merges. No known blocker. T1/T2 not run.
