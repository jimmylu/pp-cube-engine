# Research: joins in upstream Cube

Date: 2026-11-20. Upstream pin: `486f0d26` (Cube v1.8.28). Written by Claude (Anthropic) through omp for the owner's decision on the two open review findings of this change.

Two questions:

1. How does upstream Cube handle a join condition that reads a third cube?
2. Beyond a fact joined to a dimension, does upstream support fact-to-fact joins, and a fact with three or more dimension cubes?

## Method and limits

- **Source, read at the pin:** `JoinGraph.ts`, `BaseQuery.ts`, `CubeValidator.ts` and the tests of `cubejs-schema-compiler`; the Rust planner under `vendor/cube/rust/cube/cubesqlplanner`.
- **Documentation:** `docs.cube.dev` pages and the `cube-js/cube` issue tracker, read by a second agent; quotes below are verbatim.
- **Experiment A, upstream's own harness:** `vendor/cube/rust` was copied to a scratch directory outside this repository and one test module was added to the copy. It compiles models through the planner's reference join graph (`MockJoinGraph`, upstream's Rust port of `JoinGraph.ts`) and prints the SQL. `vendor/` was not touched. Cases are numbered 1A to 2l below.
- **Experiment B, this kernel:** cases 1A to 1C compiled through `pp-cube-compile` on this branch, at the code of the third review round.
- **Not done:** no SQL was executed. Experiment A uses the planner's test templates, which are Postgres-like, not Presto's. Upstream's JavaScript tests were read, not run. The production path of Cube (JavaScript bridge plus Rust planner) was not run.

## Answers in short

1. **Upstream supports it on purpose, in the JavaScript layer.** A join condition may read members of other cubes; Cube joins those cubes and places them before the join that needs them. The docs do not mention it. The logic is not in the join graph and not in the Rust planner, so this kernel, which replaces the JavaScript layer, does not have it. The planner's reference join graph does not have it either and emits the same invalid SQL this kernel does.
2. **Yes to both.** Upstream joins any cube to any cube; "fact" and "dimension" are modelling habits, not types. A fact with several dimension cubes, and chains three or four cubes deep, compile to one flat `SELECT`. Fact-to-fact joins are supported; whether the SQL stays flat depends on which cube the measures come from and on the relationship. Measures from two facts compile to one subquery per fact, merged on the shared dimensions.

## 1. A join condition that reads a third cube

### What upstream says

- **Docs: silent.** Every documented join condition reads only the declaring cube and the joined cube. No page states a rule for a third cube. ([joins reference](https://docs.cube.dev/reference/data-modeling/joins.md), [joins concept](https://docs.cube.dev/docs/data-modeling/joins.md))
- **JavaScript source: supported.** `BaseQuery.collectJoinHints` builds the join tree, collects the cubes its `ON` clauses read, puts them in front of the other join hints, and rebuilds until the tree is stable. The comment: "One cube may join the other cube via transitive joined cubes, members from which are referenced in the join `on` clauses. We need to collect such join hints and push them upfront of the joining one". `joinTreeForHints` repeats the loop for hints that arrive from the Rust planner. (`packages/cubejs-schema-compiler/src/adapter/BaseQuery.ts`, lines 602-640 and 2998-3050 at the pin)
- **It was a bug fix.** Issue [#9557](https://github.com/cube-js/cube/issues/9557): a join `ON` read members of two other cubes and they were missing from `FROM`. Fixed by [#9671](https://github.com/cube-js/cube/pull/9671): "In a join between Cube A and Cube B, sql() may reference members from other cubes. These referenced cubes must be added as join hints before Cube B".
- **Maintainers name the pattern.** Issue [#5580](https://github.com/cube-js/cube/issues/5580): third-party cubes are "cubes that will be referenced in join condition however will be neither of two joined cubes", and "Order can play a role when you're including third-party cubes in the join condition".
- **There is a test.** `describe('Transitive join paths')` in `test/integration/postgres/sql-generation.test.ts`: `test_facts` joins `merchant_and_product_dims` on `{merchant_dims.merchant_id} = … AND {product_dims.product_id} = …`, with the comment "This join depends on merchant_dims and product_dims".
- **The Rust planner rejects it for aliased joins only.** `join_tree_builder.rs`, `joined_item`: "The join `{}` of cube `{}` reads `{}`, which is neither side of the join". Upstream's test `test_rejects_a_join_reading_a_cube_outside_it` covers a join that reads a dimension whose expression reads another cube. Unaliased joins have no such check.
- **A sub-query dimension in a join condition is a separate, supported case.** The planner looks for sub-query dimensions among the members an `ON` clause reads and plans their subquery before the join (`collectors/sub_query_dimensions.rs`; fixtures `integration_subquery_in_join.yaml`, `integration_subquery_join_filter.yaml`). In upstream's 116 planner fixtures, the only join conditions that reach a third cube do it through a sub-query dimension. The baseline has no sub-query dimensions.

### Where the logic lives

| Layer | Joins the third cube | Evidence |
| --- | --- | --- |
| `JoinGraph.ts` (`buildJoin`) | No. Shortest path between the cubes it is given | Source |
| `BaseQuery.ts` (`collectJoinHints`, `joinTreeForHints`) | Yes, by iterating over the tree's `ON` clauses | Source, #9671 |
| Rust planner, main query path | No. `JoinTreeBuilder::build_for_hints` asks the join graph directly with the hints of the query's members | Source; experiment A |
| Rust planner, `try_build_for_hints` (multi-fact grouping) | Through the bridge method `try_join_tree_for_hints`, which in Cube is the JavaScript loop | Source |
| This kernel's adapter | No | Experiment B |

Not verified: whether production Cube at the pin, with the Rust planner as default, produces valid SQL for upstream's own transitive test. Read alone, the main path would not join the third cubes; the test has no skip for the Rust planner. Running it needs Node and Postgres.

### What the pinned planner emits

Model: `sales` joins `stores`; `stores` joins `regions`. Query: `sales.amount` by `stores.store_name`.

| Case | Join condition of `sales` to `stores` | Experiment A (reference graph) | Experiment B (this kernel) |
| --- | --- | --- | --- |
| 1A | reads `stores.region_key`, a plain dimension whose expression is `{regions.region_cd}` | `ON "sales".region_cd = "regions".region_cd`; `regions` is not joined | The same SQL |
| 1B | names `{regions.active}` directly | `… AND "regions".active = 1`; `regions` is not joined | The same SQL |
| 1C | reads only the two cubes | Valid | Valid |
| 1D | as 1A, and the query also selects `regions.region_cd` | `regions` is joined, but after the join that reads it | Not run |
| 1E | upstream's transitive model and query | `ON "merchant_dims".merchant_id = …`; neither third cube is joined | Not run |

So the review finding is real, it is the planner's behaviour and not a defect particular to this kernel's graph, and selecting the third cube in the query does not repair it: the order of joins is wrong too.

### What this means for the decision

- **Refusing the pattern** (a join condition may read only its two cubes, directly or through their members) has an upstream precedent: the Rust planner's own rule for aliased joins, with a message that can be reused. It makes the baseline narrower than Cube: a model that Cube compiles would be refused, and such a model cannot be compared with Cube Core at T1.
- **Supporting the pattern** means porting the JavaScript loop into the adapter: collect the cubes each `ON` clause reads, including through plain dimensions, add them before the join that needs them, and repeat until the tree is stable. Case 1D shows that the order is part of it. The added cubes must then pass this kernel's own rules: one path, one data source, measures at the root.
- The baseline's design names joins only as "a dimension through a `many_to_one` join". Nothing in it asks for third cubes in a condition.

## 2. Which join shapes upstream supports

### What the docs say

- **Relationships:** `one_to_one`, `one_to_many`, `many_to_one`, with older aliases still valid. There is no `many_to_many`; it needs a junction cube. ([joins reference](https://docs.cube.dev/reference/data-modeling/joins.md))
- **Every join is a `LEFT JOIN` and has a direction:** "All joins are directed. They flow from the source cube (where the join is defined) to the target cube". The declaring cube is the main table.
- **Path:** "Cube uses the Dijkstra algorithm to find a join path between cubes given requested members." With several paths, "this isn't guaranteed in case multiple join paths exist. Please use views".
- **Fact to fact:** the docs' own examples join cubes that both have measures, such as `orders` to `line_items` (`one_to_many`).
- **Row multiplication:** a primary key "is a requirement when a join is defined so that Cube can handle row multiplication issues such as chasm and fan traps". Cube "automatically detects chasm and fan traps based on the `many_to_one` and `one_to_many` relationships".
- **Measures of two facts:** "Each fact table gets its own aggregating subquery, and the results are joined on the shared dimensions." This is the Rust planner's feature; the docs describe it under [multi-fact views](https://docs.cube.dev/docs/data-modeling/multi-fact-views.md).
- **Limits:** none documented on the number of cubes or the depth of a chain.

Which side a join multiplies, from `JoinGraph.checkIfCubeMultiplied` and the planner's `join_multiplication.rs`:

| Relationship | Multiplied cube |
| --- | --- |
| `one_to_one` | Neither |
| `many_to_one` | The joined cube |
| `one_to_many` | The declaring cube |

Upstream's planner fixtures hold 179 joins: 107 `many_to_one`, 63 `one_to_many`, 9 `one_to_one`. In 97 of them both cubes have measures.

### What the pinned planner emits

Experiment A, a retail model: `orders` joins `customers` (`many_to_one`), `line_items` (`one_to_many`) and `shipments` (`one_to_one`); `customers` joins `regions`; `line_items` joins `products`, which joins `categories`; `returns` joins `customers`.

| Case | Query | SQL | This kernel today |
| --- | --- | --- | --- |
| 2a | One fact, three dimension cubes, one of them two joins away | Flat `SELECT`, three `LEFT JOIN`s | Compiles (golden test of this change) |
| 2b | Chain of three: `line_items`, `products`, `categories` | Flat, two `LEFT JOIN`s | Compiles |
| 2d | Fact to fact, `one_to_many`: measure of `line_items` by a dimension of `orders` | Flat; the root is `orders`, the measure is not the root's | `one_to_many` is not in the model contract |
| 2e | `count` of a cube reached through `many_to_one` | Flat, as `COUNT(DISTINCT` primary key`)` | Refused |
| 2f | `sum` of a cube reached through `one_to_one` | Flat | Refused |
| 2l | `sum` of a cube reached through `many_to_one`, by a dimension of the root | Not flat: a subquery of distinct primary keys, joined back to the cube and then summed | Refused |
| 2k | As 2l, beside a measure of the root | One subquery per cube, merged on the dimensions | Refused |
| 2c | Measures of both cubes of a `one_to_many` join | One subquery per measure cube, merged | Refused |
| 2g | Measures of two facts that share a dimension cube and are not joined to each other | One subquery per fact, merged | Refused |
| 2h | As 2g, without dimensions | Two subqueries, joined `ON 1 = 1` | Refused |
| 2i | Two paths to one cube, direct and through another cube | The direct path, chosen silently | Refused as ambiguous |

Also in upstream's suite: a chain of four cubes (`transitive_joins.rs`), and measures of three facts sharing one dimension cube (`multi_fact.rs`).

The subquery in 2l is the one the docs describe for fan and chasm traps: "a deduplication query that evaluates all distinct primary keys within the multiplied measure's cube and then joins distinct primary keys to this cube on itself".

The merge in 2c, 2g and 2k depends on the dialect (`physical_plan_builder/processors/merge/mod.rs`): a `FULL JOIN` on the dimensions when the dialect has the template `tesseract/join_types_full`, which is what the docs describe; otherwise, as in the experiment, the distinct dimension values of all subqueries combined with `UNION ALL` and each subquery `LEFT JOIN`ed back with a condition that treats nulls as equal. The second is the B1 shape the kernel design describes in appendix A, Q3.

### What this means for the decision

- **Several dimension cubes and deeper chains are B0** and already compile here. Upstream documents no limit.
- **Fact to fact is not one thing.** Grouping or filtering one fact by a dimension of another fact compiles flat upstream, like any dimension join. What leaves the flat `SELECT` is a measure whose cube the joins multiply, and measures from two cubes.
- **"Measures belong to the root" is stricter than the planner needs in two cases:** a measure through `one_to_one` (2f) and a `count` through `many_to_one` (2e) both compile flat upstream. A `sum` through `many_to_one` (2l) does not. This change refuses all three; the choice was to keep the baseline at "a dimension through a join".
- **The ambiguity rule is this project's own.** Upstream picks the shortest path and recommends views when there are several. The path search of the second review finding exists only because every path is counted here; upstream's search is Dijkstra's and cannot run long. A fix that keeps the rule needs the search bounded: follow only cubes from which the target can be reached, and stop at the second path.
- **`one_to_many` is outside the model contract** of the baseline, so case 2d cannot arise here.

## Not verified

- Whether upstream's transitive test passes with the Rust planner at the pin.
- Presto output for any case of experiment A.
- What Cube's legacy planner does for measures of two unjoined facts; the docs do not say.
- The docs' source files in the repository and the release notes were not read.
- Some 2026 maintainer comments on the cited issues are marked as generated by a coding agent; they were used for issue state and summaries only.
