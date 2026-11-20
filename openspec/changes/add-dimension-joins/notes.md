# Session notes

## 2026-11-20: proposal drafted

Drafted by Claude (Anthropic) through omp, after `add-time-dimensions` merged
as #20. The owner asked for planning and implementation in one step; no
decision in this proposal is new to the owner beyond the points listed below.

Done:
- Proposal, design, tasks and the `compile-b0` delta.
- The seed's `compile-b0` delta was re-cut in the same commit: "Dimension
  joins" moves here whole with `B0-JOIN-001` to `B0-JOIN-003`; `B0-JOIN-004`
  is new. The seed keeps "B0 boundary" with `B0-AGG-003`.

For the owner at the merge gate:
- Measures belong to the root cube; a measure of a joined cube is refused,
  under `one_to_one` too.
- That refusal and the one for unconnected cubes are query failures until the
  `unsupported` class exists.
- Cubes a join path passes through count for the data source rule.

Learned, from reading:
- Cube's join graph picks the first root that works and the shortest path;
  the rule confirmed with the seed refuses where Cube would pick.
- A `many_to_one` join multiplies the joined cube, and a measure there sends
  the planner into its multi-stage plan.
