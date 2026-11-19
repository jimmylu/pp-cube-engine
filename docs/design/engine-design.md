# Engine design

| Item | Value |
| --- | --- |
| Version | 0.4 (draft), public edition |
| Date | 2026-11-19 |
| Scope | The dataset contract, and the Rust kernel that compiles a query into physical SQL |
| Contract basis | The Cube (cube.dev) data model and query format |
| Reused open source | Cube's Rust planner `cubesqlplanner`, pinned to commit `486f0d26` (2026-11-18), Apache-2.0 |
| Engines | Presto, Apache Druid, Apache Doris |

This is the public edition of the design. It covers the contract, the kernel and baseline acceptance. Integration with any particular calling system is outside it. Implemented behaviour is recorded in `openspec/specs/`; proposed B0 behaviour remains in `openspec/changes/seed-baseline-specs/`. This design is a draft, not evidence of implemented support.

## 0 Conclusions first

1. **The design converges in steps**: fix the contract on Cube's; design the Rust crates; build what Cube already clearly supports as a baseline; only then deal with extensions.
2. **The contract is Cube's model format and query format.** The baseline narrows them and does not extend them. Any concept Cube does not have waits until after the baseline.
3. **The Rust kernel is "the upstream planner as it is, plus our own adapter layer".** This route has been tested: a standalone binary crate generated SQL through the planner's public interfaces alone, without Node (chapter 2).
4. **The adapter layer has a starting point.** Upstream's test fixtures are a pure-Rust loader for Cube YAML models and an implementation of the planner's interfaces, about 10,000 lines, which can be copied under Apache-2.0 and turned into production code.
5. **The baseline has two levels.** B0 is single-cube aggregation, filtered measures, ratios, many-to-one joins, filters and time ranges; the SQL is one flat `SELECT`. B1 is multi-fact queries, time shifts, period-to-date and more; the SQL uses common table expressions and has known limits on Druid.
6. **Rust compiles and does not execute.** Model, query and compile context in; SQL and parameters out. The caller manages models, decides access, executes the SQL and handles results.
7. **The dialect templates for the three engines are not validated yet.** This is the largest unknown in the baseline and comes first in the order of work.

## 1 Order and scope

### 1.1 Steps

| Step | Content | Output | Chapters |
| --- | --- | --- | --- |
| 1. Fix the contract | Take Cube's data model and query format as given; mark out the baseline | Model contract, query contract, compile result contract, baseline levels | 3, 4 |
| 2. Design the crates | Bring in the upstream planner; design the adapter, dialect, facade and service | Workspace layout, interfaces, thread model, upgrade policy | 5–7 |
| 3. Build the baseline | Only what Cube clearly supports, compared against Cube Core | A working kernel and acceptance cases | 8 |

Extensions beyond the baseline come after step 3 and do not feed back into it: what the baseline contract cannot express is recorded as a difference, and the baseline contract is not changed for it.

### 1.2 Scope

**In scope**: the dataset contract; the compile kernel (model + query → SQL + parameters); model validation; metadata queries.

**Out of scope**:

- SQL execution, result handling, evidence and access decisions: these belong to the caller. This document fixes only the interface.
- Cube's Node-layer features: API gateway, query orchestration, caching, pre-aggregation builds, Cube Store.

### 1.3 Decided against, or deferred

| Idea | Decision |
| --- | --- |
| Our own semantic IR, relational IR and dialect writers | Not done; the upstream planner does this |
| The kernel executing queries, holding credentials, handling results | Not done; execution stays with the caller |
| An independent validator of generated SQL; a record of where each condition came from | Deferred until after the baseline |
| Refusing SQL text at run time; fully typed expressions | Deferred. In the baseline, as in Cube, a member expression is SQL text in the dialect of its data source |
| Half-open time ranges; fixed-point post-processing | Not done. Time ranges follow Cube's semantics (3.6) |

## 2 Verified facts

These historical measurements are carried over from the source design, on macOS x86_64 with stable Rust. They were not rerun while preparing this public edition.

| Finding | How it was verified |
| --- | --- |
| The planner's own tests do not need Node | `cubesqlplanner` compiled alone and its unit tests run: 2006 passed, 0 failed, 11 ignored |
| An external crate can drive the planner through public interfaces only | A new standalone binary crate, with the upstream fixture sources included by path and one extra dependency (`petgraph`), compiled and generated SQL |
| The resulting binary does not depend on Node | It links only system libraries, has no unresolved N-API symbols, and runs with an empty environment |
| B0 queries produce flat SQL | Six queries against a demonstration model; see appendix A |
| The SQL for filtered measures, ratios, many-to-one joins and filters has the expected shape | Same runs: a filtered measure is `sum(CASE WHEN … END)`; a ratio divides after aggregating; a join is a `LEFT JOIN`; top-level filters are joined by `AND`, and an "or" group is one conjunct as a whole |
| Output is stable for the same input | The six queries gave byte-identical output across four separate process runs |
| The planner is not thread-safe | `Rc` appears 2102 times in its source, `Arc` 0 times |
| A dialect is data plus a few functions | The planner requests 64 templates by name; the templates are in Cube's JavaScript code: about 162 entries in the base set, 28 overridden for Presto, 21 for MySQL, 6 for Druid |
| The upstream fixtures are a Cube YAML model loader | About 10,000 lines of interface implementation, about 2,000 of them YAML loading, reading Cube's YAML model format |

**Not verified**:

- Templates for Presto, Doris and Druid. The runs used the fixtures' default templates, which produce PostgreSQL-style SQL.
- Generated SQL running on real clusters.
- Output compared case by case with Cube Core.
- Builds on Linux and on aarch64.
- Determinism beyond the six queries. The planner uses hash containers heavily; stable output has no structural guarantee.

## 3 Contract baseline

### 3.1 Sources of the contract

- **Model contract**: Cube's data model, in YAML or equivalent JSON. Field names, values and semantics follow the Cube documentation ([cubes](https://docs.cube.dev/reference/data-modeling/cube), [measures](https://docs.cube.dev/reference/data-modeling/measures), [dimensions](https://docs.cube.dev/reference/data-modeling/dimensions), [joins](https://docs.cube.dev/reference/data-modeling/joins), [segments](https://docs.cube.dev/reference/data-modeling/segments)).
- **Query contract**: Cube's REST query format ([query format](https://docs.cube.dev/reference/core-data-apis/rest-api/query-format)).
- **Behaviour contract**: what the planner at the pinned commit actually outputs for those inputs. Where documentation and implementation disagree, the pinned commit's behaviour wins and the difference is recorded.

### 3.2 Model contract (baseline fields)

**cube**

| Field | Baseline | Notes |
| --- | --- | --- |
| `name` | Required | Stable identifier |
| `sql_table` | One of `sql_table`, `sql` | Preferred: points straight at a physical table |
| `sql` | One of `sql_table`, `sql` | Row-level queries only; see 3.5 |
| `data_source` | Required | A cube belongs to exactly one data source, which decides the dialect |
| `sql_alias`, `title`, `description`, `public`, `meta` | Optional | `meta` is kept as it is; the kernel does not interpret it |
| `joins`, `dimensions`, `measures`, `segments` | See below | |

**dimension**

| Field | Baseline | Notes |
| --- | --- | --- |
| `name`, `sql`, `type` | Required | `type`: `string`, `number`, `boolean`, `time` |
| `primary_key` | Required on every cube that takes part in a join | Cube uses the primary key to avoid double counting caused by joins |
| `title`, `description`, `public`, `meta` | Optional | |

**measure**

| Field | Baseline | Notes |
| --- | --- | --- |
| `name`, `type` | Required | `type`: `count`, `count_distinct`, `sum`, `avg`, `min`, `max`, `number` |
| `sql` | May be omitted for `count` | Aggregating types hold the unaggregated expression; `number` holds a formula over other measures, such as `{amount} / NULLIF({order_count}, 0)` |
| `filters` | Optional | A filtered measure: each entry is one `sql` condition |
| `title`, `description`, `public`, `meta`, `format` | Optional | |

**join**

| Field | Baseline | Notes |
| --- | --- | --- |
| `name` | Required | The target cube |
| `relationship` | Required | The baseline allows only `many_to_one` and `one_to_one` |
| `sql` | Required | The join condition, such as `{CUBE}.store_cd = {dim_store.store_cd}` |

Two of Cube's rules about joins are kept as they are: every join produces a `LEFT JOIN`, and there is at most one join between a pair of cubes.

**segment**: `name`, `sql`. A predefined filter condition, referenced by name in a query.

**Member reference syntax**: `{CUBE}.column`, `{member_of_this_cube}`, `{other_cube.member}`.

### 3.3 Query contract (baseline fields)

| Field | Baseline | Notes |
| --- | --- | --- |
| `measures`, `dimensions`, `segments` | Supported | A member is written `cube_name.member_name` |
| `filters` | Supported | Leaf: `member`, `operator`, `values`; leaves can be grouped with `and` and `or`. Top-level entries are joined by "and" |
| `timeDimensions` | Supported | `dimension`, `dateRange` (absolute dates), `granularity` (`day`, `week`, `month`, `quarter`, `year`) |
| `order`, `limit`, `offset` | Supported | |
| `timezone` | Supported | Affects conversion of time dimensions |

Filter operators: `equals`, `notEquals`, `contains`, `notContains`, `startsWith`, `notStartsWith`, `endsWith`, `notEndsWith`, `gt`, `gte`, `lt`, `lte`, `set`, `notSet`, `inDateRange`, `notInDateRange`, `beforeDate`, `beforeOrOnDate`, `afterDate`, `afterOrOnDate`.

The baseline does not accept relative dates such as "last month". The caller resolves a relative period to absolute dates before submitting the query.

### 3.4 Compile result contract

| Content | Notes |
| --- | --- |
| `sql` | One `SELECT` in the target dialect |
| `params` | Parameter values in placeholder order |
| `columns` | Output columns: alias, the member it stands for, type |
| `fingerprint` | A digest of the model bundle digest, the query, the compile context and the kernel version |
| `used_cubes` | The cubes and physical tables this compilation used, for the caller's records |

### 3.5 Constraints inside the baseline

These constraints only narrow what Cube allows. A baseline model is still a valid Cube model and can be given to Cube Core unchanged.

| Constraint | Reason |
| --- | --- |
| Prefer `sql_table` | The generated SQL references the table directly, with no extra subquery |
| `sql` allows only `SELECT columns FROM one_table WHERE row_condition`: no aggregation, `DISTINCT`, window functions or joins | The Cube documentation requires a cube's query to return a plain table without aggregation; a join is declared as a join |
| A filter every query on a dataset must carry is written in the `WHERE` of the cube's `sql` | Cube supports this natively, no extra mechanism is needed, and a query cannot get around it; the cost is one more subquery level |
| Joins are only `many_to_one` or `one_to_one`, and both ends have a primary key | Adding a dimension does not change totals (appendix A, Q2) |
| Aggregation appears only in measures | Same reason |
| A member expression references only columns of its own cube and declared members | Makes validation possible |
| No fields beyond those in 3.2 | A feature outside the baseline is rejected at validation, not silently ignored |

### 3.6 Cube behaviour that is accepted

The baseline follows Cube's behaviour on these points:

- **A time range is a closed interval.** `dateRange: ["2026-10-01", "2026-10-31"]` produces `>= start AND <= end`, with the end extended to `23:59:59.999`.
- **A member expression is dialect SQL.** The expression text goes into the generated SQL as it is, so it is written in the dialect of its data source. When the same logical data has one physical source on each of two engines, the baseline models it as two cubes.
- **Row-level access arrives as filters.** The caller turns row-level access rules into filters on dimensions, and they are placed among the query's top-level filters. Top-level filters are joined by "and"; an "or" group has been observed not to wrap other top-level entries (appendix A, Q6).
- **No record of where a condition came from.** The generated SQL does not distinguish a condition that came from access rules from one the user asked for; the caller records what it passed in.
- **Default ordering is the planner's.** A query that needs a definite order states `order`.

### 3.7 Baseline levels

| Level | Features | Status |
| --- | --- | --- |
| **B0** | Single cube: `count`, `count_distinct`, `sum`, `avg`, `min`, `max`; filtered measures; `number` formulas (ratios) | Tested (default templates) |
| **B0** | A dimension through a `many_to_one` join; filtering on a joined dimension | Tested |
| **B0** | Filters (including `and`, `or`), segments, time range and granularity, ordering, row limit | Tested, except segments |
| **B1** | Multi-fact queries: measures from several cubes, combined over shared dimensions | Tested; the SQL contains `UNION ALL` inside a subquery, which Druid does not support |
| **B1** | `time_shift` on a measure (year over year, period over period) | Tested; same Druid limit |
| **B1** | `rolling_window` on a measure, including `to_date` (period to date) | Tested; depends on how a time series is generated, which differs by engine |
| **B1** | Views, detail queries (`ungrouped`), `count_distinct_approx`, custom granularities, calendar cubes | Upstream has tests; not tested in this project |
| **B1** | `compareDateRange` | The planner does not handle it; in Cube it is implemented in the API layer. When needed, the caller splits it into several compilations |
| Outside the baseline | Pre-aggregations, access policies and masking, multi-stage calculations, `case`, geo types, subquery dimensions, hierarchies | Cube supports them; this project does not use them for now |
| Outside the baseline | Concepts Cube does not have, such as dialect-independent expressions or a record of where a condition came from | Extensions, after the baseline |

B1 starts after B0 is stable and accepted.

## 4 Carriers of the contract

| Carrier | Format | Produced by | Consumed by |
| --- | --- | --- | --- |
| Model bundle | A set of Cube models (YAML or JSON) plus a version and a digest | The caller | The kernel |
| Query | Cube query JSON | The caller | The kernel |
| Compile context | Dialect, time zone, row-level access filters, request id | The caller | The kernel |
| Compile result | 3.4 | The kernel | The caller |

A model bundle is immutable and replaced as a whole by version. The model files are valid Cube models, so they can be put into Cube Core for comparison at any time.

The kernel structure, crate design, acceptance criteria and measured SQL continue in [the kernel design](engine-kernel.md).
