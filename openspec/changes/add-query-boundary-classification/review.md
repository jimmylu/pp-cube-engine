# B0 progress and route to acceptance

Assessment of main 7c20185 on 2026-11-22. Sources: main specs, remaining seed
requirements, facade/model/dialect source, service/conformance entry points,
and docs/agents/verification.md. This is a source assessment, not a new engine
test result. Seed task completion means proposal preparation, not B0 completion.

## Goal and present evidence

The goal is a compile-only kernel: Cube model + query + context -> engine SQL
and ordered parameters, with validation, security filters, metadata and stable
identity. B0 covers Presto and Doris; Druid covers single-cube B0. Callers execute
production SQL. Test infrastructure executes SQL to establish correctness.

| Area | Implemented/evidenced now | Remaining gap |
| --- | --- | --- |
| Model/query | Typed YAML/JSON, validation, B0 options | Recognized unsupported query classification; explicit immutable version/digest bundle |
| Presto compiler | Aggregates, ratios, joins, filters, dates/timezones, order/paging; T0 goldens | Cube Core result comparison and real engine/reference execution |
| Results/security | Ordered params, columns, used cubes, fingerprints, safe errors; fail-closed filters | Service must preserve these semantics across its boundary |
| Determinism/pin | Four-process successful golden replay; checkout/pin/fingerprint identity | Engine correctness is not implied |
| Doris/Druid | Target design and seed requirements | Only DialectId::Presto exists; no target templates or acceptance yet |
| Service | Entry point prints version; model metadata library exists | Compile/ValidateModel/GetMeta, version selection, safe version publication, worker dispatch |
| T1/T2 | Tier contracts exist | Conformance crate is empty; both check commands explicitly exit 2 |

PRs #37-39 strengthened acceptance coverage and identity evidence; they did not
add another engine or a working service. No defensible completion percentage
follows from PR counts, scenario counts or the seed's completed task marker.

## Concrete examples

1. Sum sales.amount by sales.channel_cd and sales.sold_at month in Asia/Tokyo,
   with a tenant dimension restriction: Presto SQL and bound values can be
   generated and checked against goldens. We have not proved that query returns
   the same rows as Cube Core or hand-written SQL on an engine.
2. The same logical query for Doris or Druid cannot yet select a corresponding
   DialectId. Model SQL expressions remain dialect-specific; portability never
   means translating arbitrary model expressions. Druid joined dimensions must
   fail Unsupported, including joins introduced only by security filters.
3. ungrouped true and hour currently fail loading; fortnight fails similarly.
   The next change distinguishes recognized unsupported behavior from a typo.
4. Alternating two Arc<Model> values is possible, but the version/digest bundle
   contract and a service request selecting model_version are absent. An invalid
   new version must never displace a usable one.
5. sales and refunds measures over a shared store dimension, time shifts and
   rolling windows remain B1. Rejecting them is B0 behavior, not unfinished B1
   implementation to squeeze into the baseline.

## Sequenced route

Each numbered item is an outcome, not necessarily one PR. Plan the next concrete
slice after evidence from its predecessor; keep harness paths isolated.

1. **Compiler boundary (next PR).** Close classification and model-first
   semantics in the existing facade; no engine/service work. Acceptance is a
   complete Unsupported matrix with all existing successful goldens unchanged.
2. **Presto execution evidence, early.** Design a pinned Cube Core comparator
   and small deterministic fixture dataset; separately build T1/T2 harness
   commands and conformance cases. Compare values, types, nulls, ordering and
   numeric tolerances explicitly. First prove an aggregate/filter/timezone
   vertical slice, then extend to the full B0 matrix. Engine/image availability
   and reference-version reproduction need investigation in that proposal.
   Do not archive full T1/T2 scenarios after a smoke test.
3. **Other dialects against that fixture discipline.** Add Doris B0 templates
   with manual-reference T2 results (MySQL compatibility is not acceptance);
   add Druid single-cube templates plus fail-closed join rejection. Use separate
   target PRs, then complete each target's declared golden/execution matrix.
   Cube Core has no native Doris dialect, so do not invent a Doris T1 result.
4. **Immutable bundles, then service.** Fix version/digest ownership, validation
   before publication, data_source-to-dialect mapping and mismatch behavior in
   dedicated proposals. Add version-isolation tests, then Compile/ValidateModel/
   GetMeta and complete context/security propagation with synchronous planner
   workers. A service is independently useful but cannot prove SQL correctness.
5. **B0 milestone.** Reconcile main specs and remaining seed requirements;
   require full dialect and execution evidence, service contract tests and
   explicit T3 milestone results where required. Only then propose B1.

An execution-first next PR would attack the largest correctness uncertainty but
requires separate infrastructure/runtime scopes and a reproducible reference.
A service-first PR would expose the kernel before engine semantics are proven.
The bounded boundary PR is recommended now; execution acceptance should follow,
rather than another series of small evidence-only closures.

Open after-baseline issues #22, #23, #30, #31, #32 and #36 remain deferred. They
must not be counted as missing B0 feature work or silently pulled into this PR.
