# Design

## Context
`crates/pp-cube-model/src/lib.rs` is one doc comment and the crate has no dependencies. The upstream fixtures load Cube YAML with `serde` structs (`src/test_fixtures/cube_bridge/yaml/` at the pinned commit). Those structs carry fields outside the baseline (`pre_aggregations`, `case`, `time_shift`, views), ignore unknown fields, lack `data_source` and the metadata fields, and build planner mocks that hold `Rc`. `serde_yaml` is only a dev-dependency of the planner, so it is not in this workspace's lockfile yet.

## Goals / Non-Goals
**Goals:** contract types every later crate can share; one loader for YAML and JSON; reference parsing and resolution; tests for `B0-MODEL-001` and `B0-MODEL-004`.

**Non-Goals:** reporting every issue with an object path (model validation), the one-source rule for a cube (`B0-MODEL-002`), bundles, anything that touches the planner.

## Decisions
- **Own types, not a copy of the fixture structs.** The fixture structs model more than the baseline and less than the contract. Copying and trimming them would cost more than writing five small structs. Alternative rejected: depend on the fixtures directly; they exist only in upstream's test builds.
- **Plain data.** Types derive `Serialize` and `Deserialize`, hold no `Rc`, and are `Send + Sync`, as kernel design 6.1 and 6.5 require.
- **One set of types for both formats.** YAML and JSON deserialize into the same structs, so equivalence is equality of the loaded values. `meta` is kept as a `serde_json::Value`.
- **Strict loading.** Structs deny unknown fields; dimension type, measure type and relationship are closed enums. Loading returns the first error with its position. Alternative rejected for this change: lenient raw types with every field optional, which pushes `Option` into every consumer. Reporting all issues with paths is left to the model-validation change, which can run over the untyped document before typed loading.
- **A cube keeps `sql` and `sql_table` as loaded.** Both are optional in the type. The exactly-one rule and its located issue belong to "Cube fields", which stays in the seed.
- **References by a hand-written scanner.** It recognises `{CUBE}` followed by `.column`, `{member}` and `{cube.member}`; any other brace content is an error. No regex dependency. Resolution checks the cube and member against the loaded model; a physical column name is taken as written, because the model does not know table columns.
- **YAML parser: `serde_yaml` 0.9**, the one the upstream fixtures use. Its author has deprecated it. If `cargo deny` rejects it, stop and open a `decision` issue naming a maintained fork; do not add an advisory exception.
- **Two implementation pull requests**, each under 400 lines: types and loading with `B0-MODEL-001`; references with `B0-MODEL-004` and the archive.
- **Validation of this design:** unit tests in `pp-cube-model` naming the two scenario ids, and `scripts/check.sh ci`.

## Risks / Trade-offs
- Strict loading reports one error, while `B0-VALID-001` wants all of them → the validation change may add an untyped pass in front of the loader; the typed API stays.
- `serde_yaml` is deprecated → the decision above; the parser sits behind `load_yaml`, so replacing it touches one function.
- YAML scalars in `meta` map onto JSON values; a YAML map with non-string keys has no JSON equivalent → it fails to load, and the test covers string, number, boolean, list and map values.
- `B0-MODEL-001` is a matrix scenario → its test exercises all four dimension types, all seven measure types and every optional field.

## Migration Plan
None; the crate has no users yet. Archiving creates the first main spec, after which `scripts/check.sh ci` fails if either scenario loses its test.
