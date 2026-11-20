# Design

## Context

PR #24 merged at the current main baseline. Model loading in `load.rs` uses strict Serde types and returns the first `LoadError`. `Model::resolve` and `parse_references` already resolve the three supported reference forms. The adapter checks query references and joins during compilation; the model crate has no public accumulated validation report.

Read: model `load.rs`, `model.rs`, `reference.rs`, `meta.rs`, `lib.rs`, loading/reference tests, the contract-model main spec and seed validation delta. The model crate already depends on Serde, JSON and YAML parsers; no SQL parser is needed for this slice.

## Goals / Non-Goals

Goals: expose complete structural diagnostics through an explicit API, preserve strict loading and valid compilation behavior, and retain stable scenario traceability.

Non-goals: certify SQL/expression restrictions, introduce a validated-model wrapper into compile, redesign errors across crates, implement service transport, or change query-dependent join rules.

## Decisions

### Next slice and alternatives

Choose structured model validation because mandatory fields and join topology otherwise fail at different boundaries. Complete compile results would also be valuable but combine columns, used sources and fingerprints. A new dialect would expand the SQL acceptance matrix. Both remain later work.

Do not combine all validation scenarios in this PR: SQL/expression constraints need separate parsing and dialect decisions. Transfer the whole structured-validation requirement and B0-VALID-001 here; leave the baseline SQL requirement and its three ids in the seed.

### Public interface

Proposed exports in `pp-cube-model`:

```rust
pub struct ValidationIssue {
    pub path: String,
    pub rule_id: String,
    pub message: String,
}
pub fn validate_yaml(text: &str) -> Vec<ValidationIssue>;
pub fn validate_json(text: &str) -> Vec<ValidationIssue>;
pub fn validate_model(model: &Model) -> Vec<ValidationIssue>;
```

An empty report means these structural checks passed; it does not certify the deferred SQL rules. Document this limitation explicitly. Keep `load_yaml`, `load_json`, `LoadError` and `compile` signatures and behavior unchanged. Callers choose validation before strict loading; automatic facade integration belongs with the later model/error boundary work.

Use JSON Pointer paths: `/cubes/0/joins/1/name`, escaping keys per pointer syntax; the empty string identifies the document root. Missing fields point to the expected field. Paths match between YAML and JSON. Messages name the definition and violated rule without dumping source SQL or metadata.

Rule ids are stable descriptive strings: `syntax`, `required`, `type`, `unknown-field`, `source`, `duplicate-name`, `reference`, `join-target`, `join-primary-key`, `duplicate-join`. Do not rely on parsing human-readable Serde error messages for paths or rule ids. Return reports in deterministic path/rule/message order.

### Validation flow

Parse documents as raw trees before strict model decoding. JSON remains strict, with comments/trailing commas disabled as in `load.rs`; preserve existing exact metadata-number rules rather than coercing values through a lossy conversion. Syntax failure produces one root `syntax` issue because no traversable document exists.

For a parsed tree, traverse objects and arrays using field rules matching the contract types. Check missing fields, scalar/collection types, enum values and unknown fields at every definition level. Continue through independently usable siblings; an invalid subtree yields a located issue without fabricated child errors. Reuse existing metadata validation behavior while preserving diagnostic paths.

Build symbols from usable cube/member names before checking references; this permits forward references. Check every available expression in dimensions, measures, measure filters, segments and join conditions. Reuse reference parsing/resolution logic where possible; do not maintain a different reference grammar. Inspect all independently parseable references so one unknown name does not suppress another. Avoid follow-on unknown-reference reports when the target's declaration itself is structurally unusable.

Semantic checks cover:

- Exactly one cube source (`sql_table` or `sql`), with a nonempty source string.
- Unique cube names and unambiguous member names, to make resolution meaningful.
- Required measure SQL except for count, including programmatically built models.
- Declared join targets; primary-key dimensions on both endpoints; one join per unordered cube pair, including repeated and reciprocal declarations.

The typed-model entry shares semantic checks with document validation. It must not claim to detect unknown fields already erased by deserialization. A raw document can carry both shape and semantic errors; collect both wherever the required inputs remain usable instead of abandoning semantic checks after the first shape failure.

Query-dependent path ambiguity, root selection, data source checks and the third-cube join-condition rule remain in the adapter. Structural validation does not add support for deferred joins.

### Acceptance and scope

Tests at the public validation entry points name B0-VALID-001. For every listed fault, test it alone, then combine independent faults into one model and assert exact paths and rule ids. Include unknown fields inside nested definitions, invalid enum values, missing keys at either join end, repeated/reciprocal joins, multiple unresolved references, forward references, typed-model mutation, malformed syntax and equivalent YAML/JSON reports.

Valid existing fixtures must validate without issues, still load identically and compile to unchanged golden SQL. Preserve metadata and strict-loader regressions. No snapshots need to change.

## Risks / Trade-offs

- Raw inspection and typed deserialization can drift: reuse field-level decoding/helpers and test the same invalid definitions against both boundaries.
- Duplicate or unusable definitions can cause misleading cascades: validate symbols before dependent checks and document suppression.
- Recovering every syntax error is outside this parser contract; aggregation applies to traversable document content.
- Tests and planning may exceed roughly 400 lines. Keep one independently verifiable structural-validation behavior together and review size before implementation.

## Open Questions

No owner-only decision is unresolved. Explicit structural validation and compatible strict loaders are the proposed API boundary; full baseline validation and automatic compile integration remain later capabilities.
