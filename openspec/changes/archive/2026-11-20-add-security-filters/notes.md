# Session notes

## 2026-11-20: next-PR planning

Prepared by Codex / OpenAI GPT-6. User requested planning and design only.

Done:
- Confirmed PR #21 merged and refreshed origin/main; started `change/add-security-filters` from that merged baseline.
- Compared security filters, model validation and complete result metadata as next slices. Recommended the missing facade security-filter path because existing B0 filters and joins provide its dependencies.
- Created proposal, design, four scenario requirements and bounded implementation tasks.
- Transferred the complete security-filter seed delta without renumbering its original three ids; added B0-SEC-004 for empty groups.
- Inspected current facade, typed filters, adapter validation and joins, existing tests and pinned planner filter ingestion. No product behavior was changed.

Open:
- Review the proposed scope, then request apply to begin implementation.
- All implementation tasks remain unchecked. No archive or main-spec change is authorized by this planning session.
- Publish the planning slice as Draft; continue implementation in the same PR after an apply request. Cross-vendor review is pending.

Learned:
- The existing context has no security-filter field; typed `Filter` values can reuse all current query checks after facade composition.
- Deserialization accepts empty groups, so security-filter validation must explicitly reject them at every depth.
- Security-only cube references must enter the same join checks as user references.
- Issues #22 and #23 remain after-baseline and outside this change. T1/T2 execution has not been run.

Verification:
- `openspec validate --all --strict`: five items passed, none failed; all four planning artifacts exist.
- `scripts/check.sh fast`: PASSED, four steps, none skipped.
- CI first failed because the sandbox prevented the advisory database lock. Authorized execution outside the sandbox passed: `check ci: PASSED — 15 steps, 0 skipped, 32s @ d5bd5ff+dirty`.
- These checks cover the planning working tree, not an exact published commit. Re-run CI on the committed head and store that evidence on the PR.
- No Rust source, snapshots, main specs, vendor, dependency or harness paths changed. Security scenarios remain proposed and unimplemented.

## 2026-11-20: implementation

User authorized apply. Implemented with Codex / OpenAI GPT-6.
- Added typed security filters to CompileContext and composed them before existing query/join validation. Caller inputs remain unchanged.
- Test-first: the initial test failed on the missing context field. The empty-group test then reproduced unrestricted SQL; recursive validation now rejects it.
- Nine facade/deserialization tests name all four scenario ids. Four new SQL snapshots were inspected and accepted using cargo insta review; existing snapshots are unchanged.
- Tests cover exact channel/category/region grouping, conflicting conditions, parameter order, mandatory cube SQL, two-hop security-only joins and prohibited join trees.
- Full compile-crate tests and strict OpenSpec validation passed. fast passed; CI passed 15 steps, none skipped (working tree evidence).
- Scope review: planning, tests and new snapshots take the PR beyond 400 lines; all implement one independently testable restriction path. No adapter, lockfile, vendor or harness changes were needed.
- Verification: completeness, correctness and coherence match the four requirements and design. T1/T2 SQL execution remains not run.
- Archived the four implemented requirements into the security-filters main spec. Next: checkpoint and run cross-vendor delivery review. Exact-head evidence belongs on PR #24. Owner merges.

## 2026-11-20: first delivery review

- Exact-head local CI and remote t0 passed. Claude review could not run because its provider quota was exhausted; DeepSeek V4 Pro produced the first completed review.
- The review's archive-timing finding conflicts with repository policy. `docs/agents/workflow.md`, lifecycle table, explicitly says: "Before final review in the same Draft PR: sync the implemented spec delta into openspec/specs/ and the folder moves to openspec/changes/archive/". Owner approval and merge happen afterward. This is the policy used here; archive is branch-local until merge.
- The controller's review prompt included AGENTS.md and the diff but omitted workflow.md because the active folder had already moved. Retain the required archive and supply this policy context in the change record for the next review.
- Corrected the proposal's artifact paths and historical planning-only language to describe implementation and archive. Re-run cross-vendor review on the revised head; no product code changed.
