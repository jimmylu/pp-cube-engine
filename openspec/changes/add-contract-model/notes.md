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
