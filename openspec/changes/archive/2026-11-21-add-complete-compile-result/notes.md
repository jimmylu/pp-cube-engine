# Session notes

## Current handoff state: stopped after bounded review repair

PR: https://github.com/jimmylu/pp-cube-engine/pull/28
Branch: change/add-complete-compile-result
Last reviewed implementation: 69f0294e47ba273b7c649be4b69f7e456d228454.
Tasks 1–6 complete; task 7 remains open. The two requirements have already been
synced into main compile-result/determinism specs and this folder is archived,
as repository workflow requires before final review. Implementation/checkpoint
publication is complete and risk:upstream is applied. Clean cross-vendor review
and final accepted exact-head evidence are NOT complete. PR remains Draft.
Owner approval and merge have not been requested.

Stopped under AGENTS.md: the review gate remained failing after two different
repair rounds. This handoff updates tracking only, without further product fixes.
Implementer: Codex / OpenAI GPT-6. Reviewer: omp / DeepSeek V4 Pro.

## Open findings and next work

The 69f0294 review reported:
- P2: task 7 was checked while notes still described final evidence as pending.
  This handoff unchecks task 7 and states the done/open boundary explicitly;
  a new review must assess the corrected record.
- P3: the gitlink guard test in fingerprint.rs requires Git and a checkout and
  duplicates the upstream pin literal. Consolidate the production pin constant,
  and make repository-only pin checking compatible with tests without Git state,
  without introducing harness changes or runtime IO into compilation.

After the remaining fix, run targeted tests and fast, checkpoint/push in this same
PR, then run delivery for exact-head CI, cross-vendor review and remote t0. Do not
mark task 7 complete or Ready until its evidence/review requirements are satisfied.
Evidence belongs in PR comments/artifacts, not an extra evidence-only commit.
No owner-only design decision or vendor/harness edit is currently required.

## Implemented behavior

The facade adds ordered output columns, used cubes/physical sources and a SHA-256
fingerprint, plus correlation-only request_id. The adapter pairs aliases from the
final SQL AST with resolved planner symbols, not query-list order. Time buckets
retain base member identity/granularity. Missing/ambiguous correspondence fails
internally. Sources are selected from rendered table factors, not graph search
candidates; quoted identifiers are decoded by the parser. CTE/derived cube sources
are rejected defensively even for direct adapter callers.

Fingerprint v1 covers canonical complete typed-model content, original query,
dialect, timezone, security filters, package version and full upstream pin. Object
keys are recursively sorted, ordered arrays and numeric metadata are preserved,
and request_id is excluded. Compile itself performs no runtime Git/IO. The pin
constant's repository test is the remaining portability finding, described above.
Raw YAML/JSON formatting is absent from the typed-model identity contract.

## Verification and repair history

Model/adapter/compile tests and all existing SQL/parameter snapshots pass. Tests
cover region grouping and ordered parameter values, exact alias/member/types,
join intermediates, quoted cube SQL/sql_alias, hidden security/time filters,
repeated output occurrences, CTE rejection, input/build fingerprint sensitivity,
canonical objects/numbers, request-id exclusion and fresh-process equality.
The pinned planner preserves both repeated selected measures; the test now
asserts exactly two columns. The all-golden four-process suite remains proposed.

Full CI and remote t0 passed on 69f0294; exact summary/review are in delivery
artifacts and PR comments. This handoff commit only receives checkpoint's fast
check unless a later CI result is explicitly recorded. T1/T2 were not run.
Strict main-spec validation passed 8 items after archive Purpose text was filled.
First review repairs added exact region/parameter/type assertions. Second repairs
added explicit repeated-column count and defensive CTE handling. Third review
found the handoff inconsistency and Git-test portability issue, triggering stop.

## Dependency and scope review

sqlparser reuses =0.64.0 with std/visitor, without optional stack extension. SHA-256
uses sha2 0.11.1. Cargo deny passed advisories, bans, licences and sources.
Cargo.lock changed; risk:upstream is applied and upstream pin is unchanged.
Size exceeds 400 lines including tests/dependencies, retained as one successful
result behavior. No vendor, harness or existing golden snapshot edits occurred.
