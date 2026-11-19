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
- **YAML parser: `serde_norway` 0.9**, a fork of `serde_yaml` 0.9, which the upstream fixtures use. Checked on main with `cargo deny --locked check`: `serde_yaml` 0.9 and `serde_yaml_ng` 0.10 fail on RUSTSEC-2024-0449 (`unsafe-libyaml` is unmaintained); `serde_norway` 0.9.42 and `serde-saphyr` 2.2.0 pass. In a throwaway test `serde_norway` loaded a sample model equal to its JSON form and reported object path and position for an unknown field and an unknown type; it adds two packages to the lockfile. Alternative rejected: `serde-saphyr`, written in safe Rust, adds twelve packages and by default read the plain scalar `yes` in `meta` as boolean true. Alternative rejected: an advisory exception in `deny.toml` for `serde_yaml`.
- **JSON parser: `serde_json_lenient` 0.2 with its `float_roundtrip` feature**, a fork of `serde_json` with the same interface; `serde_json` stays for the `Value` type and for writing. Stock `serde_json` rounds some floats when parsing (`1.2699041298821259`, `1.5e-30`), so a model loaded differently from JSON than from YAML and changed on a round trip. Alternative rejected by the owner: `float_roundtrip` on `serde_json` itself, because features are unified across the workspace and it would change how the planner's `serde_json` is built. Checked with `cargo metadata`: the fork adds one package and changes no other package's features, and `cargo deny` passes. The fork accepts trailing commas and comments by default; `load_json` switches both off. Alternatives rejected: `struson`, which switches on a `serde_core` feature; `json5`, which accepts more than JSON.
- **Two implementation pull requests**: types and loading with `B0-MODEL-001`, about 480 lines, which the owner accepted so that the review fixes land together; references with `B0-MODEL-004` and the archive. Each type arrives with the test that uses it, so the join and segment types are in the second.
- **Validation of this design:** unit tests in `pp-cube-model` naming the two scenario ids, and `scripts/check.sh ci`.

## Risks / Trade-offs
- Strict loading reports one error, while `B0-VALID-001` wants all of them → the validation change may add an untyped pass in front of the loader; the typed API stays.
- `serde_norway` still parses with libyaml translated to unsafe Rust (`unsafe-libyaml-norway`) → the parser sits behind `load_yaml`, so replacing it touches one function; if it gains an advisory, stop and open a `decision` issue instead of adding an exception.
- YAML scalars in `meta` map onto JSON values; a sequence or map used as a key has no JSON equivalent and fails to load, while a scalar key such as `1` is read as a string key → the test covers string, number, boolean, null, list and map values.
- A `meta` number is kept only when it is exact: an integer within the 64-bit range, or a finite float below 2^63 in magnitude. Anything else (`.nan`, `.inf`, a longer integer, which a JSON parser would silently round) fails to load in both formats and is written as a string instead. This narrows "preserve `meta`" for values that no JSON value can hold; found in review.
- `B0-MODEL-001` is a matrix scenario → its test exercises all four dimension types, all seven measure types and every optional field.

## Migration Plan
None; the crate has no users yet. Archiving creates the first main spec, after which `scripts/check.sh ci` fails if either scenario loses its test.
