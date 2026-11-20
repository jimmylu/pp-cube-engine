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

The owner agreed to the three open points on 2026-11-20: the adapter is written
against the contract types, Presto templates are transcribed only for this
path, and the pull request is large.

Learned:
- The planner's Rust entry is `State` and `TopLevelPlanner`; `BaseQuery` needs
  a Node context.
- Presto does not override `params/param` or `quotes/*`; it inherits `?` and
  `"` from the base set and overrides `statements/select`.
- Upstream's fixtures are compiled only for its own tests, and `serde_yaml`
  and `insta` are its dev-dependencies, so neither is in this lockfile.

## 2026-11-20: implemented

By Claude (Anthropic) through omp, test first. All tasks.

Tracer (task 1.3), one `sum` grouped by one dimension:
- It linked and returned SQL on the first run; the test binary links against
  system libraries only. Nothing under `vendor/` was touched.
- Templates the planner asked for: `statements/select`,
  `statements/group_by_exprs`, `expressions/column_aliased`,
  `expressions/query_aliased`, `expressions/order_by` rendered;
  `quotes/identifiers` and `quotes/escape` read raw. A distinct count adds
  `functions/COUNT_DISTINCT`, a boolean dimension
  `expressions/wrap_boolean_dimension`. The other aggregates are not templates.
- Bridge methods called: on the evaluator `cube_from_path`, `measure_by_path`,
  `dimension_by_path`, `is_measure`, `is_dimension`, `is_segment`,
  `cube_exists`, `pre_aggregations_for_cube_as_array`; on the tools
  `driver_tools`, `sql_templates`, `get_allocated_params`, `all_cube_members`,
  `compile_member_sql` and `try_join_tree_for_hints` (not
  `join_tree_for_hints`); on the driver only `should_reuse_params`.
  `parse_path` was not called.
- The planner adds an order of its own: the first measure descending, or the
  first dimension ascending when there is no measure.

Also done: the two golden scenarios; a cube defined by `sql`; query and
internal failures; archive. `cargo deny` passes and no existing package's
resolved features change.

Not as planned:
- `insta` is held at 1.43. The latest release changed a feature of `bitflags`,
  which the planner depends on.
- A query naming a member the model lacks is refused before planning; the
  planner's message is `Cannot resolve: <member>` without the cube.
- A query over two cubes fails as internal. It should become unsupported or a
  join when those requirements are adopted.
- Indexing into the model by positions found in the same model is kept; it
  cannot fail, since the model is immutable behind its `Arc`.

Open: delivery; execution of the SQL on Presto has never happened (T1, T2).

Learned:
- With resolver 2, `cargo metadata` shows features with dev-dependencies
  included, so a test-only dependency can still change what the planner is
  built with under `cargo test`.
- An optional bridge method needs a `has_<name>` companion; the compiler lists
  the missing ones.
