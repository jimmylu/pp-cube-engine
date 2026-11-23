# Using the kernel

The kernel turns a Cube data model, a Cube query and a compile context into SQL
and parameters for Presto or Apache Doris. It never executes SQL. What it
supports is the B0 baseline described in [the design](design/engine-design.md);
implemented behaviour is recorded in [openspec/specs](../openspec/specs).

## The three inputs

- **Model**: Cube's data model in YAML or JSON, narrowed to the baseline. A
  field outside it is an error, never ignored. Validate before use. A measure
  may roll over time (`rolling_window`, also `type: to_date`), be another
  measure at another time (`multi_stage` with `time_shift`) or estimate a
  distinct count (`count_distinct_approx`). A time dimension may declare
  `granularities` of its own.
- **Query**: Cube's REST query format: measures, dimensions, segments, filters,
  time dimensions with absolute date ranges, order, limit, offset, timezone,
  and `ungrouped` for rows instead of totals.
- **Context**: the dialect, a default timezone, a request id, and the security
  filters the caller derived from its access rules.

## As a library

```rust
use std::sync::Arc;
use pp_cube_compile::{CompileContext, DialectId, PreparedModel};

let model = Arc::new(pp_cube_model::load_yaml(model_text)?);
// Validates and digests once. Keep it and compile many queries against it.
let prepared = PreparedModel::new(model)?;
let query = pp_cube_model::load_query(r#"{"measures":["sales.amount"]}"#)?;
let compiled = prepared.compile(&query, &CompileContext {
    request_id: "r-1".into(),
    dialect: DialectId::Doris,
    timezone: "UTC".into(),
    security_filters: vec![],
})?;
```

`pp_cube_compile::compile` does the same for a one-off compilation, validating
the model on every call. The planner is not thread-safe internally, but a
`PreparedModel` is `Send + Sync` and compilations on different threads share
nothing else, so compile on as many threads as there are cores.

## As a service

See [the service guide](../crates/pp-cube-service/README.md) for running,
securing and operating the gRPC service, and its `proto/engine.proto` for the
contract. A container image builds from the repository root:

```sh
scripts/vendor-sync.sh
docker build -t pp-cube-engine-service .
docker run --rm -p 50051:50051 -v "$PWD/model.yaml:/model.yaml:ro" pp-cube-engine-service \
  --model v1=/model.yaml --data-source warehouse=doris --listen 127.0.0.1:50051
```

The image contains the binary and no shell, and runs as a non-root user.
`crates/pp-cube-service/examples/compile.rs` is a minimal client that compiles
one query over TLS with a token and prints the SQL, parameters and session
properties. A
loopback address inside a container is reachable only from that container; to
publish the port, listen on `0.0.0.0` with TLS and tokens as the service guide
describes.

## Using a result

- **Bind the parameters.** `sql` contains `?` placeholders; `parameters` are
  their values in order, each typed. Never splice a value into the SQL.
- **Apply the session properties.** `session_properties` lists engine settings
  the SQL needs. Presto needs `offset_clause_enabled=true` for a query with an
  offset; without it the engine rejects the statement.
- **Expect one statement, not one shape.** A query over one fact is a single
  `SELECT`. A query over several facts, or a sum over rows a join repeats,
  begins with `WITH`: one common table expression per aggregation.
- **Execute unchanged.** Run the SQL as returned, on the engine of the dialect
  you asked for. With the service, SQL is only returned when that dialect
  serves every data source the query reads.
- **Read the columns.** Each output column names its alias, the member it
  stands for, its type and, for a time dimension, its granularity.
- **Cache on the fingerprint.** It covers the model content, the query, the
  dialect, timezone, security filters, kernel version and upstream pin, and
  excludes the request id.

## Security filters

Row-level access arrives as Cube filters in the context. The kernel appends
each as a top-level condition, so a user's `or` group can never widen them, and
fails the compilation when one cannot be applied. The kernel does not decide
access: whoever calls it chooses the filters, so only the system that enforces
access should be able to call it.

## Failures

| Kind | Meaning | What to do |
| --- | --- | --- |
| model | The model is invalid or outside the baseline | Fix the model; issues name the path and rule |
| query | The query is wrong for this model or dialect | Fix the query; the message names the member or rule |
| unsupported | A recognized Cube feature the kernel or the dialect does not implement | Do not retry; the feature is not available |
| internal | The kernel or planner failed | Report it; the message carries no detail by design |

No failure returns SQL, and no message contains table names or SQL text.

## Compared with Cube Core

For every execution case, Cube Core 1.8.28 (the release of the pinned upstream
commit) was asked for the SQL of the same model and query on Presto. Its SQL
and parameters equal the kernel's in all sixty-five cases, with one difference:

- **Default row limit.** Cube's API adds `LIMIT 10000` to a query that sets no
  limit. The kernel adds none: a query without a limit compiles without one.
  A caller that wants a cap states `limit` in the query.

Two things Cube does in its API layer are the caller's here:

- **Security filters.** Cube has no separate channel for them in a REST query;
  the comparison appends them to the query's top-level filters, which is where
  the kernel puts the filters of the compile context.
- **Timezone.** The kernel takes a default from the compile context; Cube takes
  it from the query. The comparison states the effective timezone in the query.

Joins were compared separately, with the same result under both of Cube's
planners:

- Measures of a cube joined one_to_one, and count or count_distinct of a cube
  joined many_to_one, compile to the SQL Cube writes.
- A join whose condition reads a third cube joins that cube first, as Cube does.
- Sum, avg, min and max of a cube reached through a many_to_one join are
  aggregated over distinct keys, in the multi-stage SQL Cube writes.

Queries over several facts were compared the same way, forty-nine in all:

- Measures of two or three cubes over shared dimensions, with dimension
  filters, segments, measure filters, order and paging, compile to the SQL Cube
  writes. So do a measure beside one of a cube joined one_to_one, and a formula
  over measures of both.
- A fact that cannot reach a dimension or filter of the query is an error in
  Cube and a query error here.
- **A filter on a count over repeated rows** is unsupported here. For a count
  of a cube reached through a many_to_one join, Cube selects the count of
  distinct keys but compares the count of joined rows in the filter, so a
  filtered result can contradict its own column.

Rolling windows, period to date and time shifts were compared in fifty-three
more queries. What both compile is the same SQL. The kernel refuses, naming the
measure, some queries Cube compiles:

- **No time dimension.** Cube returns a windowed measure as a total over all
  time and a shifted measure unshifted. So it does for a shift along a time
  dimension the query does not have.
- **Weeks and quarters on Presto.** Cube writes `interval '1' week`, which
  Presto rejects. A window or shift of weeks or quarters, and a rolling window
  by week or quarter, are unsupported on Presto.
- **A rolling range that starts inside a period, on Presto.** Cube's series
  starts where the date range starts, so a range from the 15th by month gives
  periods from the 15th. Start the range at the start of a day, month or year.

Ungrouped queries, approximate distinct counts and declared granularities
were compared in twenty-five more queries, with the same result. Again the
kernel refuses some things Cube compiles:

- **An interval of several units without an origin** is a model error. Cube
  counts such periods from the start of the current year, so its SQL changes
  with the calendar; the kernel reads no clock.
- **A granularity counted from an origin, over a range that starts before
  it.** Cube's expression puts every row before the origin one period late.
  Give the time dimension a date range that starts at the origin or later.
- **A rolling window by a declared granularity**, and **an ungrouped query
  with measures that need an aggregation of their own** (several cubes, rows a
  join repeats, rolling or shifted), where Cube's result is partly grouped.

Cube Core has no Doris dialect, so Doris output is accepted by execution only.
On Doris the periods of a rolling window are written out one by one, aligned
to the granularity: every unit and any range compile, but rolling by a
granularity needs a date range.

## Not supported yet

Apache Druid, views and relative date ranges. See [performance](performance.md) for latency figures.
