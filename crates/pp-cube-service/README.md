# Local gRPC service

Run validated model versions on an explicit local socket:

```sh
cargo run -p pp-cube-service -- \
  --model v1=model.yaml --model v2=model-next.yaml \
  --listen 127.0.0.1:50051 --data-source warehouse=doris \
  --workers 2 --queue 16
```

`--model version=path` is repeatable; each file is UTF-8 YAML or JSON and every
one must be valid, or nothing starts. `--data-source name=dialect` is repeatable
and names the dialect (`presto` or `doris`) that serves each data source; every
data source of the models needs one. The listen address must be a literal
loopback address; port zero chooses a free port. Worker and queue capacities
must be positive; their defaults are 2 and 16.

## Model versions

A version is identified by the service's digest of its typed content, `sha256:`
and 64 hex digits. The digest does not change with formatting, key order or
YAML versus JSON; ValidateModel returns it for a valid model.

With `--allow-publish`, versions can also be managed while serving:

- `PublishModel` validates a model and makes the version available. Versions
  are immutable: the same content again succeeds and creates nothing, different
  content is refused. State `expected_digest` to publish only that content.
- `RetireModel` stops offering a version; compilations already running on it
  finish. `ListModels` returns the available versions and digests.

Without the flag the three operations are refused, and at least one `--model`
is required. Published versions are held in memory: after a restart only the
`--model` files are available, so the host republishes (publishing is safe to
repeat). Anyone who can publish decides the SQL every caller receives; enable
it only on a socket the host controls.

After validation and binding, stdout prints `listening <actual-address>`.
Startup failure writes a diagnostic to stderr and exits unsuccessfully. Ctrl-C
stops the server gracefully. Clients use `ppcube.engine.v1.CubeEngine` from
`proto/engine.proto`: Compile, ValidateModel and GetMeta. Compile names exactly
one available version and supplies dialect, timezone and trusted security filters.
The service compiles SQL without executing it.

A compiled response carries the SQL, one typed value per placeholder to bind in
order, the output columns, the cubes and tables read, a fingerprint and the
engine session properties the SQL needs: apply every one before executing (on
Presto a query with an offset needs `offset_clause_enabled=true`). SQL is
returned only when the requested dialect serves every data source the query
reads. Every response for an available version names the kernel version, the
upstream pin and the model version and digest.

This process is for trusted local callers, who can choose security context. It
has no authentication or TLS. To integrate remote access, use the library's
`GrpcService` in a host that owns those controls. 
