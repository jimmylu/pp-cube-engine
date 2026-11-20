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

## 2026-11-20: review, round 3; stopped

Stopped under AGENTS.md: the review failed again after two different fixes.
The reviewed code is the commit before this note; the pull request is a Draft;
local CI passes on that code. Nothing below is fixed.

Codex found two P2 in `crates/pp-cube-adapter/src/joins.rs`. Neither has been
reproduced here; both read as correct.

1. A join condition can read a third cube. If `sales` joins `stores` on a
   member of `stores` whose expression reads `regions`, a query over `sales`
   and `stores` gets a tree of those two, and the planner writes the `ON`
   condition with an alias for `regions` that is not joined: invalid SQL, and
   `regions` escapes the data source rule.
2. `walk` enumerates every chain of joins when the target cannot be reached.
   The limit of two found paths never applies then, because nothing is found.
   A deep model with two joins per cube makes an ordinary "not connected"
   query run for a very long time.

Proposed, for the owner to choose before anyone continues:
- For 1, either refuse a join condition that reads any cube but the two it
  joins, which is a rule about models and the smaller change, or follow the
  condition's references and require those cubes in the tree. The first is
  recommended; it needs a sentence in the requirement and a scenario.
- For 2, search only through cubes from which the target can be reached,
  found by one pass over the joins backwards. Then every step leads to the
  target and the limit of two applies. No decision needed.

Learned: three rounds found three different gaps in one file. The graph's
rules were built by cases; before another fix, write down what a tree
guarantees (every alias the SQL names is joined, on one data source, by one
path, with the measures at the root) and check each against how the planner
uses the tree.

Open: the two findings, then delivery. The change is archived in this branch;
`openspec/specs/compile-b0/spec.md` already holds the requirement, so the
branch must not merge as it is. The SQL has not been executed (T1, T2).

## 2026-11-20: research on upstream joins

Asked by the owner as the basis for deciding the two open findings. The
findings are in `research-joins.md`, beside this file; no code changed.

- Finding 1 is reproduced on this branch and is the planner's behaviour:
  upstream's reference join graph produces the same SQL. Cube joins the
  cubes a join condition reads in its JavaScript layer, which this kernel
  replaces. The Rust planner refuses the pattern for aliased joins only.
- Upstream joins any cube to any cube. Several dimension cubes and chains
  compile flat. A measure through `one_to_one`, and a `count` through
  `many_to_one`, also compile flat upstream; this change refuses both.
- Counting every path is this project's rule; upstream takes the shortest.

Corrected: the stop record above named a commit by its hash, which the
spelling check rejects, so the pushed commit of that record fails `t0`.
The hash is replaced by words.

## 2026-11-20: the owner's decisions

1. A join condition that reads a third cube: refuse it now and record it;
   support it later, at lower priority. Recorded as the requirement "Join
   conditions" (`B0-JOIN-005`); support is issue #22.
2. Measures through a join that the planner compiles flat: keep refusing
   them; support later, at lower priority. Already refused by `B0-JOIN-004`;
   support is issue #23.

The deferred requirements are issues labelled `after-baseline`, not seed
requirements: the seed is the B0 backlog and is closed out before B1, and
the owner gave these a lower priority than that. Each issue holds the
requirement and a draft scenario; the change that adopts one assigns its id.

## 2026-11-20: the two fixes

By Claude (Anthropic) through omp. Both findings were reproduced by a failing
test before the fix: the first returned SQL naming a cube it did not join;
the second had not answered after twenty seconds.

- Join conditions: a tree that needs a join whose condition reads a cube
  other than its two is refused, with the planner's own wording for aliased
  joins. The condition is followed through member expressions.
- Path search: one breadth-first chain, then one search per join of that
  chain with the join left out. The walk over every chain is gone. The deep
  model now answers within the test run's hundredths of a second.
- design.md has a table of what a join tree guarantees and where each point
  is held. Before the fix, every other place an expression can read another
  cube was probed: segments, measure filters, measure and dimension
  expressions, one and two joins away. The planner joins the cubes for all
  of them; only join conditions were open.

Open: delivery. The SQL has not been executed (T1, T2).
