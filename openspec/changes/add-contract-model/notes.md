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
- The branch is stacked on the seed proposal commit, so the pull request can
  only be opened after the seed proposal has merged, and then needs a rebase.
- The project config was not on this branch when the artifacts were written;
  its rules were applied by hand.

Open:
- Gate 1 for this proposal, and a review by a vendor other than Anthropic.
- Whether `cargo deny` accepts `serde_yaml` 0.9 is unknown until task 1.1.

Learned: the upstream fixture loader is `serde` structs over `serde_yaml`, a
dev-dependency of the planner only, so it adds to this workspace's lockfile.
