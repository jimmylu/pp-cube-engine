# Design

## Context

PR #21 is merged. The facade currently accepts `CompileContext { dialect, timezone }` and passes the borrowed query straight to the adapter. `Filter` already supports typed conditions and nested groups; its deserializer rejects mixed shapes and unknown operators but accepts empty groups. Adapter validation checks members, operator/type compatibility and value counts. Join validation sees query filters and follows member expressions.

Sources inspected: `pp-cube-model/src/query.rs`, `pp-cube-compile/src/lib.rs`, adapter `schema.rs`, `options.rs`, `joins.rs`, compile filter/join tests, and pinned planner `query_properties_compiler.rs::compile_filters`. The planner consumes the options' filters through its existing filter compiler. The public kernel design section 6.4 already places security-filter composition in the facade.

## Goals / Non-Goals

Goals: preserve every supplied restriction as a top-level conjunct or return an error; reuse query validation and join planning; retain ordered parameters and immutable inputs.

Non-goals: decide who may access rows, interpret policy, execute SQL, build service transport, change general user-filter semantics, or implement deferred joins and dialects.

## Decisions

### Sequence and alternatives

Security filtering is the recommended next slice: aggregation, filters, time and joins already exist, making the missing context path independently testable. Model validation is a useful alternative but spans parsing and SQL restrictions. Complete result metadata is another alternative but needs column, source and fingerprint contracts. Both remain in the seed backlog.

For composition, use the facade as the public design specifies. Adapter-only injection would require every check and join calculation to learn a second filter input. SQL post-processing would risk incorrect grouping and parameter placement. Neither is needed.

### Interface and data flow

Add `security_filters: Vec<pp_cube_model::Filter>` to `CompileContext`, with Rustdoc specifying top-level AND semantics and empty-list behavior. Existing callers add `Vec::new()`. Do not expose raw JSON or a policy evaluator in the facade.

Before planning, recursively reject empty security groups, clone the query and append cloned security entries after its user entries. Submit that effective query to the existing `plan` path. Keep group boundaries, values and entry order; do not deduplicate, overwrite matching user members, mutate caller input or retry without filters.

Data flow: model + user query + context filters → effective query → existing member/type checks → existing join checks and planner → SQL + ordered parameters, or error.

Use the ordinary Cube filter semantics, including WHERE for dimensions and HAVING for measures. Callers express row restrictions on dimensions. Groups mixing row and aggregate predicates follow the existing planner's acceptance or rejection; no new policy meaning is inferred.

### Failure and join guarantees

| Input | Required handling |
| --- | --- |
| Empty security list | Preserve current SQL and parameters |
| Empty AND/OR at any depth | Query error before planning |
| Unknown member, incompatible operator, invalid values/date | Existing validation error; no SQL |
| Restriction on a cube absent from user selections | Include its declared join path or fail |
| Ambiguous/disconnected/cross-source path, forbidden join condition | Existing join refusal; no SQL |
| Planner/internal failure | Existing internal error; no SQL |

Malformed JSON cannot construct a `Filter`; test this at the existing deserialization boundary without adding a new context loader. Programmatically constructed invalid conditions still reach validation. Errors use today's Query/Internal classes; broader classification remains separate.

Only narrowly scoped adapter changes justified by a failing security test belong here. Stop if correcting a finding needs a broader contract change or harness edit.

### Scenario transfer and tests

Move the complete seed `security-filters` file here with its original three requirements and ids. B0-SEC-004 adds an explicit empty-group consequence of failing closed. Keep the seed folder because other capabilities remain. Main specs stay unchanged during planning.

Add `tests/security.rs`, reusing sales and retail fixtures; add only fixture fields needed for the seed's channel/category/region example. Tests name scenario ids:

- B0-SEC-001: user OR plus region restriction, nested groups, multiple security entries, matching user/security members with conflicting values, parameter correspondence and input immutability. Inspect SQL grouping; data-level execution remains unverified at T0.
- B0-SEC-002: `valid_sales` with no query filters retains `WHERE is_valid = 1`; also retain it when user and context filters exist.
- B0-SEC-003: malformed deserialization, unknown member, incompatible operator/value count, and security-only cube references. Cover a successful two-hop dimension restriction and refusals for disconnected, ambiguous and cross-source paths.
- B0-SEC-004: empty AND/OR at the root and nested beside a valid condition; empty context list preserves output.

New snapshots may describe added security predicates and joins. Existing snapshots must remain unchanged; run `cargo insta review` for new snapshots and commit no pending files. Inspect predicates and parameters together rather than relying only on predicate substrings.

## Risks / Trade-offs

- Adding the context field breaks struct-literal callers; update compile tests in the same behavior change.
- Cloning the query adds allocation; acceptable for this bounded B0 path without an unmeasured optimization.
- T0 proves SQL shape and failure behavior, not engine execution or end-to-end authorization. T1/T2 remain not run.
- Existing planner checks may reject a complex restriction. Failing is preferable to returning SQL without that restriction, as the seed requires.
- Target one behavior around 400 changed lines. Planning artifacts count toward that review; review scope before implementation, without splitting its correctness fixes.

## Open Questions

No unresolved owner-only decision. The recommended sequence and empty-group interpretation are explicit proposals for review. Planning does not approve or implement product behavior.
