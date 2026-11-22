# Design

## Context

See proposal.md. At main 7c20185, Query and TimeDimension deny unknown fields,
Granularity has only five variants, and model/query tests intentionally refuse
hour at load time. The facade validates Model before security filters,
fingerprinting and adapter planning. Its public input is Arc<Model>, not raw JSON.

Pinned references: cubeclient's `V1LoadRequestQuery` has `ungrouped: Option<bool>`;
planner fixture `cube_bridge/yaml/dimension.rs` lists second/minute/hour plus
the five B0 units. compareDateRange is an API-layer operation in the public design,
not a planner feature; this change must never send it to the planner.

## Goals / Non-Goals

Expose a finite typed boundary, preserve grouped behavior and model-first
compilation. No open-ended extension map, replacement query parser, raw-JSON
compile entry point, extra result forms or B1 planning.

## Decisions

1. Add optional `ungrouped: Option<bool>` to Query and
   `compare_date_range: Option<Vec<DateRange>>` to TimeDimension, with absent
   fields omitted in serialization. Add Second/Minute/Hour enum variants.
   Explicit false is accepted; comparison presence, including an empty list,
   is unsupported. DateRange keeps its existing absolute-pair validation.
   `null` has existing Option semantics (absence); no new null policy is added.
   This is deliberately a classification carrier, not full comparison execution.
2. Add a small query-boundary guard after full model validation and before
   security checks, fingerprinting and planning. It returns Unsupported for true
   ungrouped, comparison presence, or any sub-day granularity. Model errors win
   over these recognized options. Syntax/type errors remain load failures
   before compile can be called; model-first does not promise raw JSON precedence.
   Keep existing multi-cube-measure checks and prove their public error class.
3. Keep the adapter safe when invoked directly: unsupported new options must
   return PlanError::Unsupported before building planner options. The facade
   maps that class consistently. Prefer one reusable typed boundary predicate
   over divergent option lists; do not expose planner details in the model crate.
4. Preserve serialization for absent new fields so ordinary fingerprints and
   goldens do not drift. Explicit false may have a distinct fingerprint because
   it is an explicit typed query input; generated SQL/parameters must be equal.
5. Transfer the seed's entire two requirements now, deleting their empty files.
   Update only the date/granularity requirement in main at final sync; preserve
   both existing ids. B0-QUERY-011 is the next free query id at proposal creation.

Alternatives: leave parser rejection unchanged (violates the seed contract);
add a raw JSON compile API (duplicates loading and expands scope);
implement sub-day and detail output (crosses B0). The finite typed carrier keeps
one compile API and provides the required distinction.

## Risks / Trade-offs

- Public struct/enum source break → document exact fields/variants in the PR;
  update all workspace constructors, retaining Default construction behavior.
- Existing tests enforce the old hour rejection → update both model and compile
  tests, preserving unknown-name tests rather than deleting their assertions.
- A test id could conceal an incomplete matrix → B0-AGG-003 covers all four
  feature families and each sub-day unit; B0-QUERY-009 pairs every recognized
  boundary case with an invalid-model run and a valid-model run.
- Accidental feature acceptance → test facade and direct adapter failures, with
  no SQL result and no dropped query options. Unknown fields still fail loading.

## Migration Plan

Implement in the same Draft PR after the owner requests apply. Run targeted
loading, boundary, adapter and fingerprint tests, then all existing goldens and
CI. No snapshot update is expected. Verify, sync and archive before final
different-vendor review; owner approves the final head and merges.
Reverting restores the old loading behavior; no persistent data migration.
