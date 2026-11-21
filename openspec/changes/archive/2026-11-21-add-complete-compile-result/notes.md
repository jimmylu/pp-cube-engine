# Session notes

## Current handoff state: resumed repair; final delivery pending

PR: https://github.com/jimmylu/pp-cube-engine/pull/28
Branch: change/add-complete-compile-result
User authorized continuing after the bounded stop. Tasks 1–7 cover completed
implementation, sync/archive and checkpoint publication; risk:upstream is applied.
Exact-head acceptance is a separate final delivery gate recorded in PR comments
and artifacts, so recording evidence does not create another unreviewed commit.
Clean cross-vendor review and accepted final-head CI/t0 evidence remain pending
at this checkpoint. PR remains Draft. Owner reviews and merges after Ready.
Implementer: Codex / OpenAI GPT-6. Reviewer: omp / DeepSeek V4 Pro.

## Review repairs and next work

The 69f0294 review's tracking contradiction was corrected in ce32819. This resumed
repair separates completed archive/publication tasks from the exact-head delivery
gate explicitly, rather than checking a task that claims future evidence.
The Git-test portability finding is repaired: one UPSTREAM_PIN constant feeds
production identity and the repository test. The test skips source archives
without .git and missing Git executables; other process failures and gitlink
mismatches still fail. Compilation itself performs no Git or filesystem IO.

`cargo test -p pp-cube-compile --locked` passed after the repair. Running the exact
gitlink unit test with PATH=/nonexistent also passed with an explicit skip reason.
Final-head fast/CI, cross-vendor review and remote t0 are next; their SHA-bound
results belong in PR evidence. T1/T2 were not run. No owner-only design decision
or vendor/harness edit is required.

## Baseline and scope evidence for the resumed review

The review on 3ea01c5 raised two conditional findings. Neither requires a product
change or an expanded scope:

- `retail.yaml` predates this PR. `git ls-tree
  8e85004bdd40dc6ffc724b0af61557f576a79a71
  crates/pp-cube-compile/tests/fixtures/retail.yaml` returns mode 100644 and blob
  `2cfdfdb80a6a48ed9f2843bf0ddba4f4ad2e328b`. The new test reuses that tracked
  baseline fixture; it does not need to add it again. Full local CI and remote t0
  passed on 3ea01c5, including compilation of this include_str! expression.
- This approved change's proposal explicitly lists transfer of the entire
  Complete compile result / Fingerprint identity requirements from the seed,
  and its Impact includes the two seed deltas. Removing those same requirements
  from seed after syncing them to main specs prevents duplicate ownership; no
  other seed behavior is implemented or edited. This is bookkeeping within
  add-complete-compile-result's stated scope, not implementation of another
  change. Classified errors and all-golden repeatability remain in the seed.

The fixture and scope evidence is supplied for cross-vendor reassessment; the
3ea01c5 review remains a failed gate until a fresh accepted report is published.

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
constant's repository test is conditional on Git metadata and executable availability.
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
