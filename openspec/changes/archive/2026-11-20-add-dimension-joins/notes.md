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

## 2026-11-20: implemented

By Claude (Anthropic) through omp, test first. All tasks.

Done:
- Adapter: a join graph over the model's joins (`joins.rs`), join definitions
  on each cube, and a check before planning that the query's cubes form one
  tree whose root owns the measures.
- Templates added: `statements/join`, `tesseract/join_types_left`. The
  planner asked for no other.
- A fixture with six cubes and two golden snapshots. No existing snapshot
  changed; `Cargo.lock` is unchanged.

What the planner does, seen in the snapshots:
- It reads each cube's joins from the cube definition and asks the join
  graph for the tree of the query's cubes.
- Joins are written in the order the graph returns them, each as
  `LEFT JOIN <table> AS "<cube>" ON <condition>`, in the one flat `SELECT`.
- A segment or a filter of a joined cube lands in the same `WHERE`.

Changed on the way:
- The test that showed an internal failure hiding its detail used a query
  over two cubes, which is now a query failure with a message. It tests the
  error's text directly instead.
- A model with a join to a cube it does not declare now fails every query
  against it, naming both cubes, until model validation reports it earlier.

## 2026-11-20: review, round 1

Codex (OpenAI) found one P2: the check before planning looked only at the
members the query names. A dimension whose expression reads another cube
brings that cube into the join, and a measure beside it could then be
compiled from a joined cube. Fixed: the check follows member expressions
through the model's references; a test covers a dimension and a formula that
read another cube.

## 2026-11-20: review, round 2

Codex found one P2 in the same check: a count without an expression counts
the primary key, and a primary key that reads another cube joined that cube
unseen. Reproduced with a failing test first. Fixed where the planner's cubes
are known: the join graph refuses a tree that holds the measures' cube and
does not start from it. The check before planning stays for its messages.

Learned: do not mirror the planner's reasons for joining a cube; judge the
cubes it asks the graph for.

This is the second fix for the same review finding area. A third finding
there is a stop condition.

Open: delivery. The SQL has not been executed (T1, T2).
