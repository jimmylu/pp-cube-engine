# Design

## Context

See proposal.md for motivation. Today `scripts/engines/doris-container.py` and
`presto.py` split `.snap` files and Rust Debug output to recover SQL and
parameters. The Doris temporal test in `pp-cube-compile` compiles live but pipes
cases to a Python file under `crates/`, which starts a second container. The
seed design places T1/T2 tests in `pp-cube-conformance`. Harness paths travel
alone, so the emitter must exist before a tier can call it.

## Goals / Non-Goals

**Goals:** one seam between product and harness: the product declares cases and
expectations; a tier boots an engine and runs a manifest. The same shape serves
Doris, Presto and Druid T2 and, later, Cube Core comparison.

**Non-Goals:** a general fixture language; a Rust engine client; changing how
the tiers run in this pull request.

## Decisions

- **Manifest as the interface.** `cargo run -q -p pp-cube-conformance --bin
  execution-cases -- doris` prints JSON on stdout. Alternatives: keep parsing
  snapshots (couples tiers to insta and Debug formats, cannot vary parameters
  without text replacement); let each Rust test spawn its own verifier (one
  engine start per suite, control inverted). A binary rather than an ignored
  test keeps stdout clean and gives the harness a stable command.
- **Shape.** `{"schema":1,"dialect":"doris","cases":[{"name","scenario",
  "fixture":[statement...],"sql","params":[{"type":"text","value":...}],
  "columns":[...],"rows":[[cell...]]}]}`. Cells are JSON numbers, strings, null,
  or `{"timestamp":"2026-10-01T00:00:00"}`. Fixture statements are dialect SQL
  text; consecutive cases with an identical fixture reuse the loaded data.
- **Parameters are typed.** Every `Parameter` variant maps to a declared type or
  the emitter fails (B0-EXEC-002), replacing the current panic on non-text.
- **Expectations are literals** written beside each case, never derived from
  compiled output.
- **Cases live in Rust** next to the model fixture they compile, reusing the
  conformance crate's own copy of the sales model; `pp-cube-compile` test
  support is not a dependency.
- **Transition.** `real_doris_temporal_execution` keeps its ignore attribute and
  its B0-DORIS-010 comment, but obtains cases from the conformance emitter;
  `doris_time_engine.py` becomes a generic manifest runner on the existing
  container helper. Both are deleted by the change that archives B0-DORIS-010.

## Risks / Trade-offs

- A circular dev-dependency (compile tests reading conformance) → the test
  invokes the emitter command rather than linking the crate.
- Manifest and tier drift → `schema` is checked by the runner; a T0 test pins
  the Doris manifest's case names and scenario ids.
- Scope near 400 lines → migration is limited to the thirteen temporal cases.
- A T0 test naming B0-EXEC ids proves the manifest, not execution; execution
  ids remain uncovered until a tier runs them.

## Migration Plan

Additive. Revert the pull request to return to the self-contained opt-in test.
Evidence for this pull request: `scripts/check.sh ci`, the standard Presto and
Doris container tiers, and the explicit temporal command on local Docker Doris.
