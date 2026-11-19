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

## 2026-11-20: local review of the first implementation
Reviewed by Codex (OpenAI GPT-6), two independent axes; implementer: Claude (Anthropic).
Scope: local 657481d...8dbdf2e, tasks 1.1, 1.2 and 1.4; remote PR #14 was not fetched.
Standards: no additional untooled violations or meaningful code smells found.
Spec P2, load.rs:30: metadata 1.2699041298821259 changes in JSON loading and breaks YAML equivalence and round trips.
Spec P2, load.rs:23: YAML metadata 18446744073709551617 fails; JSON accepts it with precision loss.
Spec P2, model.rs:48: YAML metadata .nan, .inf and -.inf silently become null; reject unrepresentable values.
Three temporary regression probes confirmed these failures; no product files were changed.
Command: `scripts/check.sh ci` on 8dbdf2e.
All seven loading tests pass; preceding CI steps, including cargo deny and strict validation, pass.
Summary: `check ci: FAILED at "traceability"`; B0-MODEL-001 is unknown to the main-spec-only check.
Action: fix metadata-number handling and add regressions; resolve the separate harness blocker through #13.
Open: all three findings, #13 and later-PR tasks; task 3.2 is not complete. No push or remote comment.
Learned: the exactly representable 0.5 fixture cannot detect numeric metadata conversion loss.

## 2026-11-20: review findings
By Claude (Anthropic) through omp.
- Finding 1, fractions differ between JSON and YAML: fixed in this pull request.
  `serde_json` now has `float_roundtrip`, and the fixtures use
  1.2699041298821259, so the equivalence and round-trip tests fail without it.
  The feature applies to every crate in the workspace build, the planner
  included; it only makes float parsing exact.
- Findings 2 and 3, integers beyond 64 bits and `.nan`/`.inf`: fixed together
  with task 1.3 in the second pull request, to keep this one under 400 lines.
  Until that merges both defects remain. The rule is in design.md.
- #13 was decided as option A; the harness change is pull request #14. Open
  this pull request only after #14 has merged and the branch is rebased.
