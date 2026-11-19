# Session notes

## 2026-11-20: proposal drafted

Drafted by Claude (Anthropic) through omp, at the owner's request to start
implementation. No product code was written: the workflow allows none before
this proposal has merged.

Done:
- Proposal, design, tasks and the `contract-model` delta.
- The two requirements were moved out of the seed in the same commit: they are
  deleted from `seed-baseline-specs/specs/contract-model/spec.md`, which keeps
  "Cube fields" and "Immutable model bundles".

State:
- Rebased onto main after the seed proposal and the project config merged; the
  artifacts were checked against the config rules.
- The YAML parser was chosen by a throwaway probe on main, not committed:
  `cargo deny --locked check` with each candidate, and a test loading one model
  from YAML and JSON. Results are in design.md.

Open:
- Gate 1 for this proposal, including the parser choice, and a review by a
  vendor other than Anthropic.

Learned:
- The upstream fixture loader is `serde` structs over `serde_yaml`, a
  dev-dependency of the planner only, so a YAML parser adds to this lockfile.
- `serde_yaml` 0.9 fails `cargo deny` here (RUSTSEC-2024-0449), so the parser
  upstream uses cannot be adopted without an advisory exception.
- Make every commit before opening the pull request: a push after the gate
  reaches the approval wait leaves the new gate run queued behind the old one.

## 2026-11-20: first implementation pull request

Implemented by Claude (Anthropic) through omp, test first.

Done (tasks 1.1, 1.2, 1.4):
- `pp-cube-model`: contract types for model, cube, dimension and measure;
  `load_yaml` and `load_json`; fixtures and tests for `B0-MODEL-001`.
- `cargo deny` passes with `serde_norway`; two packages are new in `Cargo.lock`.

Changed from the plan, see tasks.md and design.md:
- Three implementation pull requests instead of two, to stay under 400 lines.
- The join, relationship and segment types moved to the references pull request.

Open:
- Task 1.3 was written and passed locally, then left out for size. Its shape:
  `LoadError` gains `position: Option<Position { line, column }>`, from
  `serde_norway`'s `location()` and `serde_json`'s `line()`/`column()`; the
  measure rule is checked in a hand-written `Deserialize` for `Measure` that
  wraps the derive through `#[serde(remote = "Self")]`.
- Review by a vendor other than Anthropic.

Learned: `rustfmt` at its default width spreads struct literals in tests over
many lines; budget test code generously when sizing a pull request.

## 2026-11-20: stopped at the traceability step

`scripts/check.sh ci` fails on this commit with `Tests naming an unknown
scenario: B0-MODEL-001`; every step before traceability passes. The step reads
ids only from `openspec/specs/`, and this change's delta is archived in its last
pull request, so a test cannot name its scenario before then. Rule 4 asks for
the id on the test now. The owner decides how to resolve this in issue #13.

The branch is pushed without a pull request. Do not open one, and do not remove
the scenario comments, until #13 is decided.
