# Design

## Context

Main 0673ee4 contains the successful-result work from PR #28. CompileError has
Model(Vec<ValidationIssue>), Query(String), Internal { detail }; PlanError has
User and Internal. From<CubeError> preserves the upstream user/internal cause.
Joins::check currently emits CubeError::user for multiple measure-owning cubes,
including dependencies followed through expressions. Existing joins tests
therefore expect Query for this recognized out-of-B0 feature.

The compile facade accepts typed Query, not raw JSON. Model::load_query rejects
unknown fields before compile runs. This change does not invent a parse facade.
The seed's transfer policy explicitly permits moving complete requirements in
the implementing change. Its compile-result delta contains only Classified
errors, so that file is deleted after transfer. Main specs stay untouched until
implementation and meaningful tests are complete.

## Goals / Non-Goals

Provide all four failure classes promised by B0-RESULT-002 while preserving the
successful-result contract. Cover public Display messages and existing model
issues; Internal.detail and derived Debug remain operator diagnostics, not a
safe response format. Do not change the model issue API or implement B1.

## Decisions

### Typed failure at the detecting boundary

Add Unsupported(String) to PlanError and CompileError. In Joins::new/check,
propagate the existing multiple-measure-owner rejection as a typed unsupported
failure. Keep CubeError at upstream bridge trait boundaries and preserve its
user/internal cause; do not change upstream or encode class markers in strings.
The adapter's entry-point checks can return PlanError directly and use its
From<CubeError> conversion for existing fallible helpers. Keep the private join
trait methods' CubeError signatures unchanged.

Use the existing dependency-aware owner detection, including filter/security
measure dependencies and number formulas. Do not infer unsupported solely from
the number of names in query.measures. Unknown-member checks run before this
detection. Different-data-source queries remain Query per B0-JOIN-003; give that
check precedence if a request also contains measures from multiple sources.
Ambiguous/disconnected joins remain Query. A recognized multi-owner request
within one data source is Unsupported and never reaches B1 SQL planning.

Alternatives: classifying message substrings is fragile under upstream wording
changes; a separate facade-only query scan duplicates dependency resolution.
Returning every rejection as Query preserves today's shape but fails the spec.

### Caller-safe diagnostics

Preserve messages authored by local checks when they contain member names and
rules only. Audit upstream-origin User messages: do not forward arbitrary SQL or
physical identifiers to the facade. Use a safe rule-level fallback for those
messages unless their structured provenance guarantees safety; do not attempt
SQL redaction with a regular expression. Keep original internal details only in
the existing operator-only Internal.detail field. Unsupported messages name the
baseline restriction and logical members/cubes, not physical sources.

Retain Model issue paths/rule ids and the generic Model Display message. Retain
the generic Internal Display message. Do not make Debug a public response format.
Model validation still precedes query/security checks and unsupported detection.
Compilation failure carries no successful SQL/result fields.

### Scope and tests

Transfer Classified errors / B0-RESULT-002 verbatim. The scenario is exercised by
public invalid-model, unknown-member and same-source multi-fact requests, plus a
unit test of the real planner-error conversion seam for a non-user failure. If
no valid B0 input deterministically triggers an internal failure, inject a
CubeError into that private mapping seam in a unit test; add no public failure
switch or fake production planner. Test safe public messages using distinctive
neutral physical table/column/SQL markers in the fixture and error payload.

Update only the existing multi-fact rejection assertions to Unsupported. Keep
cross-source, disconnected, reached-cube-measure and unknown-member assertions
as Query; add overlap/precedence coverage. Verify dependency-only multi-owner
requests and unchanged successful compilation. Existing golden snapshots must
not change, including the compile fingerprints from unchanged typed inputs.

## Risks / Trade-offs

- New enum variant breaks exhaustive consumers: document it in the proposal and
  update workspace matches, without changing the compile signature.
- Over-classifying upstream User errors: only locally recognized baseline
  restrictions become Unsupported; other user causes stay Query.
- Leaking upstream diagnostics: exercise the actual conversion and Display seam,
  not only manually constructed public errors.
- Growth beyond 400 lines: review scope before publication; keep one error
  classification behavior together and avoid unrelated validation rewrites.

## Migration Plan

Use the same Draft PR for proposal, implementation, archive and final delivery.
Run targeted adapter/facade tests, strict specs, fast, then full CI. Sync/archive
only after the scenario is implemented; cross-vendor review and remote t0 must
pass on the exact final head before Ready. Owner approves and merges. Reverting
this change restores the former three-class API without changing successful SQL.
