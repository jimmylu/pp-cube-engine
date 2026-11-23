# Using the kernel

The kernel turns a Cube data model, a Cube query and a compile context into SQL
and parameters for Presto or Apache Doris. It never executes SQL. What it
supports is the B0 baseline described in [the design](design/engine-design.md);
implemented behaviour is recorded in [openspec/specs](../openspec/specs).

## The three inputs

- **Model**: Cube's data model in YAML or JSON, narrowed to the baseline. A
  field outside it is an error, never ignored. Validate before use.
- **Query**: Cube's REST query format: measures, dimensions, segments, filters,
  time dimensions with absolute date ranges, order, limit, offset, timezone.
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
| unsupported | A recognized Cube feature beyond B0 or the dialect | Do not retry; the feature is not available |
| internal | The kernel or planner failed | Report it; the message carries no detail by design |

No failure returns SQL, and no message contains table names or SQL text.

## Not supported yet

Apache Druid, multi-fact queries, time shifts, rolling windows, views, ungrouped
queries, approximate distinct counts, custom granularities and relative date
ranges. See [performance](performance.md) for latency figures.
