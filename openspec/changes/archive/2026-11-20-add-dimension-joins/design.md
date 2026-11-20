# Design

## Context
The adapter's join graph answers for one cube only; for any second cube it fails with "not available". A cube definition tells the planner it has no joins. The model type holds each cube's joins: the joined cube's name, `many_to_one` or `one_to_one`, and a condition such as `{CUBE}.store_cd = {stores.store_cd}`. Loading does not check that the joined cube exists.

Read from the pinned planner and its test fixtures, not run:
- The planner hands the join graph the cubes of a query and expects a root, the joins in the order they are written into SQL, and for each cube whether the joins multiply its rows. A join is an edge with the declaring cube, the joined cube, the relationship and the condition as an expression of the declaring cube.
- Cube's own join graph tries each cube as the root, takes the shortest directed path to each other cube, and prefers the tree with the fewest joins. It picks silently between candidates.
- A cube is multiplied when a `many_to_one` join leads to it. A measure of a multiplied cube sends the planner into its multi-stage plan, which is B1 SQL.
- A join renders through `statements/join` with the join type from `tesseract/join_types_left`.
- The planner also asks, through the tools, for a join tree that may not exist and takes "none" for an answer.

## Goals / Non-Goals
**Goals:** the requirement of the delta on Presto: a fact grouped and filtered by dimensions of joined cubes in one flat `SELECT`, and the four refusals.

**Non-Goals:** measures of more than one cube or of a joined cube; model validation of joins; join paths chosen by the caller; filters from the compile context.

## Decisions
- **The root is the one cube of the query from which every other cube of the query is reached** by following joins in the direction they are declared. No such cube: the query fails, naming the cubes. Two such cubes, which needs joins declared in both directions: ambiguous. Alternative rejected: Cube's preference for the first candidate, which chooses silently.
- **Every path counts, not only the shortest.** If two different chains of joins lead from the root to a cube, the query is ambiguous and fails naming the root and that cube. The owner confirmed this rule with the seed; it is stricter than Cube, which takes the shorter chain.
- **Two paths are found without listing paths.** The graph takes one chain to the target by breadth-first search; a second chain exists exactly when the target is still reached with some join of the first one removed. That is one search per join of the chain. Alternative rejected after review: walking every chain, which does not end in reasonable time on a deep model when the target cannot be reached.
- **A path may pass through cubes the query does not name;** they are joined too. Joins are written in the order they are found from the root, so each is joined after the cube it starts from.
- **All cubes of the tree share one data source,** the ones passed through included. Otherwise the query fails naming the cubes and their data sources. Data source names are model names, not physical structure.
- **Measures belong to the root.** A measure of any other cube fails the query, naming the measure. A `one_to_one` joined cube is not multiplied, so its measures could be compiled flat, and so could a count through `many_to_one`; they are refused all the same, because the baseline has "a dimension through a join" and no more. The owner confirmed the refusal on 2026-11-20 and deferred the two flat cases to issue #23. Measures from two cubes can never both be at the root, so they are refused by the same rule.
- **These refusals are query failures for now.** `B0-AGG-003` wants `unsupported` for measures from more than one cube; that class arrives with `compile-result`, which then reclassifies. The delta says "fail without SQL" for the two cases, so it will not need to change.
- **A join to a cube the model does not declare fails the query, naming both cubes.** It is a model fault; model validation will report it earlier once it exists.
- **The check for the root runs before planning, on everything the query reads:** the members it names and, followed through their expressions, the members those refer to in any cube. It gives the messages that name a measure.
- **The graph holds the planner to the same rule.** The planner can join cubes the check cannot foresee, such as the cube that the primary key of a count reads. The graph knows the cube of the query's measures, and any tree it builds that holds this cube must start from it. This is judged on the cubes the planner actually joins, so it does not depend on knowing why it joins them. The path and data source rules apply there too.
- **A join condition reads only its two cubes.** A tree that needs a join whose condition reads another cube, directly or through a member, is refused, naming the join and the cube. Cube supports such conditions by joining the other cubes first, in its JavaScript layer; the pinned planner alone names the cube in the `ON` clause and never joins it (research-joins.md). The owner chose the refusal on 2026-11-20 and deferred support to issue #22. Alternative rejected for now: porting Cube's loop, which also has to order the joins. The rule is applied per query, to the joins of its tree, so a model that holds such a join still serves the queries that do not need it.
- **Golden tests:** one snapshot for `B0-JOIN-001`, one for a dimension two joins away with a time dimension and a segment of the joined cube. The refusals are asserted by their messages.
- **Validation:** the golden tests, tests for each refusal naming the five scenario ids, a test that a joined cube's measure is refused under either relationship, a test that a deep model with two joins per cube answers at once, `scripts/check.sh ci`.

### What a join tree guarantees
Written after three review rounds each found a gap; every later change to the graph is checked against this list.

| Guarantee | Where it is held | What the planner does with it |
| --- | --- | --- |
| Every cube the SQL names is joined | Members: the planner collects the cubes of every member a query reads, through expressions too, and asks the graph for all of them (probed: segments, measure filters, measure and dimension expressions, one and two joins away). Join conditions: the rule above | Renders one `LEFT JOIN` per edge, in the order given |
| A cube is joined after the cube its join starts from | Edges are taken along each chain from the root | Same |
| One path to each cube, one root | `tree`, by the search above | None; the planner takes any tree |
| One data source | `tree`, over the root and every joined cube | None |
| The measures' cube is the root | `check` before planning for the message; `build` for every tree the planner asks for | Would otherwise plan a multiplied measure |

## Risks / Trade-offs
- The facts above come from reading → the first task compiles `B0-JOIN-001` end to end before the rules are built.
- The planner may ask for joins in ways the reading missed → every refusal in the graph is a user error with a message, and the tracer shows what is asked.
- Stricter than Cube: a model with a shortcut join beside a chain compiles in Cube and is refused here → by design; the message names the cubes so the model or the query can be changed.
- The size is above the guide → named in proposal.md.

## Migration Plan
No type changes. A query over one cube compiles as before. On archive, `openspec/specs/compile-b0/spec.md` gains the two requirements.
