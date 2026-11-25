# Python client

A client of the compile service for Python 3.9 and later: a Cube query in, SQL
and its parameters out. It is the generated gRPC stubs of the service's
contract and a small layer over them. It executes nothing.

```python
from ppcube import Client, CompileRefused

with Client('engine.internal:50051', token=read_token,
            trust='/etc/portal/engine-ca.pem') as kernel:      # omit trust for the system's authorities
    compiled = kernel.compile(
        'property/2026-11', 'presto', query,                  # a dict in Cube's format, or JSON text
        timezone='Asia/Shanghai',
        security_filters=[region_rule],                       # row-level rules of the caller's user
        request_id=request_id)

    cursor = warehouse(session_properties=compiled.session_properties).cursor()
    cursor.execute(compiled.sql, list(compiled.parameters))   # never write a value into the SQL
```

## What you get back

`compile` returns a frozen `CompiledQuery`:

| | |
| --- | --- |
| `sql` | One statement with `?` placeholders. Run it unchanged, on the engine of the dialect asked for |
| `parameters` | One value per placeholder, in order: `str`, `float`, `bool` or `None` |
| `sql_as(paramstyle)` | The statement for a driver of another DB-API style; see below |
| `session_properties` | Engine settings the SQL needs. Apply every one before executing |
| `columns` | The result's columns: `alias`, the model `member` it stands for, its `type`, its `granularity` |
| `fingerprint` | A key for caching, as hex. It covers the model content, query, dialect, timezone, security filters and kernel version, so it changes with every kernel version |
| `model_digest`, `kernel_version` | What produced the SQL |

## Drivers and placeholders

The kernel writes `?`. Drivers differ in what they accept:

| Driver | Engine | Use |
| --- | --- | --- |
| `prestodb` (presto-python-client) | Presto | `cursor.execute(compiled.sql, list(compiled.parameters))` |
| `pymysql`, `mysqlclient` | Doris | `cursor.execute(compiled.sql_as('format'), compiled.parameters)` |
| A driver of `numeric` style | | `compiled.sql_as('numeric')` |

`sql_as('format')` writes `%s` for each placeholder and doubles every other
`%`; a `?` or `%` inside quoted text or a quoted identifier stays what it was.
If it cannot follow the statement it raises rather than guess. Do not replace
placeholders yourself, and never format a value into the text.

## When there is no SQL

- `CompileRefused`: the service answered, without SQL. `.kind` says whose it is
  to fix: `model`, `query`, `unsupported` (do not retry) or `internal`. The
  message names members and rules, never SQL or tables.
- `grpc.RpcError`: the call failed. `.code()` is `NOT_FOUND` for an unknown
  model version, `UNAUTHENTICATED` for a missing or wrong token,
  `RESOURCE_EXHAUSTED` for a full service (back off); `UNAVAILABLE` and
  `DEADLINE_EXCEEDED` are worth one retry.

## Rules a caller keeps

- **Security filters are the caller's decision.** The kernel applies what it is
  given, where the query cannot widen it. Derive them from who the user is;
  never accept them from the user. The read token is therefore a trusted
  credential: it belongs to your back end, not to a browser or a notebook
  shared with others.
- **One client, shared.** It holds one connection and may be used from several
  threads. With `multiprocessing` or a forking server, make the client after the
  fork: a gRPC channel does not survive one.
- **Encrypted, with a token**, unless the service is on the same host:
  `plaintext=True` is for loopback, and a token is never sent over it.

## Building

```sh
pip install clients/python          # from a checkout, with pip 21.3 or later
```

The stubs are generated from `crates/pp-cube-service/proto/engine.proto` of
the checkout by `generate.py`, which the build runs; it needs `grpcio-tools`.
There is no copy of the contract here and the generated files are not kept in
the repository, so the package builds only from a checkout, and carries the
kernel's version. A pip older than 21.3 copies the package away from the
contract before building it; with one, run `python clients/python/generate.py`
first. It is not published to an index yet. To work on the client
itself: `python clients/python/generate.py`, then run the tests with
`PYTHONPATH=clients/python/src python -m unittest discover -s clients/python/tests`.

`meta` returns the generated `GetMetaResponse`; the other operations of the
contract are reachable through `ppcube._contract.engine_pb2_grpc`.

A complete, runnable use is in
[examples/insurance-analytics](../../examples/insurance-analytics):
`./run.sh python`.
