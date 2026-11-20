# Session notes

## 2026-11-20: proposal drafted

Drafted by Claude (Anthropic) through omp. Planning only; no product code.

Done:
- Proposal, design, tasks and the `compile-b0` delta.
- The requirement "Single-cube aggregation" was moved out of the seed in the
  same commit; the seed's `compile-b0` keeps "Dimension joins" and "B0 boundary".
- The facts in design.md were read from the pinned planner and from Cube's
  JavaScript at the pinned commit by two read-only passes. Nothing was built or
  run, so task 1.3 confirms them before anything depends on them.

Open, for the owner before implementation:
- The adapter is written against the contract types instead of copying the
  upstream fixtures, which the kernel design proposes.
- Presto templates are transcribed only for the path this change renders.
- The pull request will be about 1,500 lines.

Open otherwise: pull request #17 must merge first; `parse_references` is in it.

Learned:
- The planner's Rust entry is `State` and `TopLevelPlanner`; `BaseQuery` needs
  a Node context.
- Presto does not override `params/param` or `quotes/*`; it inherits `?` and
  `"` from the base set and overrides `statements/select`.
- Upstream's fixtures are compiled only for its own tests, and `serde_yaml`
  and `insta` are its dev-dependencies, so neither is in this lockfile.
