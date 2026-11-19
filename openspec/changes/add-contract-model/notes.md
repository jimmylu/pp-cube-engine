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

Implemented by Claude (Anthropic) through omp, test first. Tasks 1.1 to 1.4.

Done:
- `pp-cube-model`: contract types for model, cube, dimension and measure;
  `load_yaml` and `load_json`; fixtures and tests for `B0-MODEL-001`.
- `LoadError` carries a position; a measure other than `count` without `sql`
  fails to load.
- `cargo deny` passes; three packages are new in `Cargo.lock`.

Changed from the plan:
- The join, relationship and segment types move to the references pull request,
  so each type arrives with the test that uses it.
- The first commit failed the traceability step, which then read ids only from
  `openspec/specs/`. The owner chose option A in #13; the harness change is #14.

Learned: `rustfmt` at its default width spreads struct literals in tests over
many lines; budget test code generously when sizing a pull request.

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

## 2026-11-20: review findings resolved
By Claude (Anthropic) through omp. Each finding was first reproduced as a failing test.

Owner decisions, in conversation on 2026-11-20:
- A `meta` number is kept only as an integer within the 64-bit range or a finite
  float below 2^63 in magnitude; anything else fails to load and is written as a
  string.
- Nothing may change how the upstream planner's dependencies are built; the
  JSON parser is `serde_json_lenient`.
- All three fixes go into this pull request. It is about 700 lines; the first
  estimate of 480 was wrong, and the owner confirmed the real size.

Fixes:
- Finding 1: stock `serde_json` rounds some floats on parsing, 1.5e-30 as well
  as the reviewer's fraction. Its `float_roundtrip` feature fixes that but would
  also apply to the planner's `serde_json`. JSON is now parsed by
  `serde_json_lenient`, a separate package with the same feature, with its two
  extensions switched off. Both floats are in the fixtures.
- Findings 2 and 3: `meta` has its own deserializer (`src/meta.rs`) applying the
  rule above in both formats.

Open: review of these fixes by a vendor other than Anthropic (task 3.2); group 2.

Learned: to see whether a dependency change touches the planner's build, compare
each package's resolved features from `cargo metadata` before and after. Adding
`serde_json_lenient` changed none; `struson` switched on a `serde_core` feature.
