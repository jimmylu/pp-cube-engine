# Views

## Why

A view gives the people who query a model one name for members of several
joined cubes, and the model's owner a place to decide what is exposed. It is
what Cube recommends as the public face of a model. The kernel refused the
field. This is the rest of the eighth unit of the productization route, split
from #85.

## What Changes

- **Model.** A model may declare `views`: a name and, per cube along a join
  path, the members to take (`includes` as names or `*`, `excludes`,
  `prefix`). Before: an unknown field, a model error.
- **Queries on views.** View members are compiled as the members they take,
  under the view's column names, as Cube writes them; they carry every
  capability their members have, and every refusal.
- **Service.** Compile accepts queries on views; GetMeta lists views after the
  cubes.
- **How.** A view is expanded to a cube whose members refer to the taken
  members by join path (`{sales.stores.store_name}`), which is what Cube's
  compiler hands the planner. The reference form is internal: a model
  expression that writes one is a model error.
- Five execution cases run on Presto, on Doris and against Cube Core.
- Baseline level: B1. New goldens only; no existing golden changes.

Non-goals: members declared on a view itself (measures, dimensions,
segments); taking segments; `alias`, folders, hierarchies and default filters
of views; choosing between several join paths by the view's path (the model's
one-path rule stays); access policies.

## Capabilities

### New Capabilities

None.

### Modified Capabilities
- `contract-model`: views (B1-MODEL-003).
- `model-validation`: view rules (B1-VALID-004).
- `compile-b1`: queries on views (B1-VIEW-001 to -003).
- `dialect-doris`: views on Doris (B1-DORIS-004).
- `service-api`: views in Compile and GetMeta (B1-API-001).

## Impact

Paths: `crates/pp-cube-model` (`model.rs`, `reference.rs`, `validation.rs`,
`sql_validation.rs`), `crates/pp-cube-adapter` (`schema.rs`, `expression.rs`,
`joins.rs`, `windows.rs`), `crates/pp-cube-compile`, `crates/pp-cube-service`,
tests of the four crates, `crates/pp-cube-conformance` (model, cases,
goldens), `README.md`, `docs/guide.md`, `CHANGELOG.md`, five specs and this
change folder. Callers: the Rust type `Model` gains the public field `views`
and `Reference` a variant; `{a.b.c}` in an expression is now reported by
validation as a reference fault rather than as malformed braces.
