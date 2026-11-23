# Presto execution

## Scope

The Presto tier executes the live-compiled execution manifest of
`pp-cube-conformance` on Presto 0.293. It reads no stored snapshot and holds no
fixture or expectation of its own. The kernel remains compile-only: only this
tier executes SQL. Cases, fixtures, expected rows, the declared session
properties and their reviewed golden are product files in
`crates/pp-cube-conformance`; change them there, in a product change. The same
cases run on Doris; see [doris-container.md](doris-container.md).

A pass is Presto T2 for the scenarios the final line names (B0-PRESTO-003). It
says nothing about Cube Core comparison (T1, not built), Druid or real clusters.

## Run locally

Delivery does not start this container; the `presto-smoke` job of the `engines`
workflow runs it on GitHub (see [delivery.md](delivery.md)). Run it locally when
developing cases or debugging, with a Docker daemon and amd64 support/emulation
and the pinned Rust toolchain:

```sh
scripts/check.sh engines-container
```

Before any container is created the tier runs
`cargo run -q --locked -p pp-cube-conformance --bin execution-cases -- presto`,
so a compiler failure never starts an engine. It then pulls a digest-pinned
Presto 0.293 amd64 image, provisions one uniquely named container with a
loopback-only ephemeral port and 2 GiB limit, installs the memory catalog,
checks the exact runtime build, probes Memory worker execution, executes the
manifest and removes the container. Progress lines show each stage without
publishing raw server responses.

For an already prepared disposable engine with the memory catalog
(`connector.name=memory`) installed before startup:

```sh
PRESTO_URL=http://127.0.0.1:8080 scripts/check.sh engines
```

Without PRESTO_URL, `engines` exits 2 (NOT RUN); protocol, query or result
failures exit 1. Each fixture drops and recreates its tables in `memory.dw`, so
use a disposable engine only.

## How a case runs

Each distinct fixture is loaded once, statement by statement over HTTP, for the
consecutive cases that share it. The session uses catalog `memory`, schema `dw`
and time zone UTC; the SQL applies named zones itself.

The compiled SQL travels unchanged, with its `?` placeholders, as a prepared
statement in the `X-Presto-Prepared-Statement` header, form-URL-encoded as the
pinned Presto client does. `EXECUTE ... USING` carries one literal per manifest
parameter: text as an apostrophe-escaped string, number as a `DOUBLE` literal,
bool as `TRUE`/`FALSE`, null as `NULL`. Placeholder substitution is never used.

Session properties the manifest declares go into `X-Presto-Session`. Presto
declares `offset_clause_enabled=true`: PrestoDB rejects the compiled `OFFSET`
clause without it, and a caller of the kernel must set the same property
(B0-PRESTO-017). The tier adds no property of its own.

Column names and ordered rows must equal the case's literal expectation. An
expected timestamp is compared in the protocol's `YYYY-MM-DD HH:MM:SS.mmm` text
form. An unknown manifest schema, dialect, case field, session shape, parameter
type or cell shape fails the run.

## Bounds, cleanup and diagnostics

Each HTTP query has a 30-second POSIX wall-clock deadline including body reads,
503 retries and best-effort cancellation. Redirects, cross-origin continuation,
query errors, changing columns and excessive rows fail. Public failures use
fixed allowlisted diagnostics rather than endpoints, SQL or server messages.

Docker commands have explicit timeouts. Version readiness uses runtime.nodes,
because this build rejects version(). The pinned image reports 0.293-2f48ff6.
Memory worker readiness uses a bounded CTAS probe; only known no-nodes errors
are retried, each attempt on a distinct table. Cleanup is attempted after any
create attempt, even a timeout, and its failure fails the run. SIGKILL or
shutdown can leave an owned cube-presto- container requiring manual removal;
pre-existing containers are never removed.

## Sources and history

- Protocol: https://github.com/prestodb/presto/blob/0.293/presto-docs/src/main/sphinx/develop/client-protocol.rst
- Header encoding: https://github.com/prestodb/presto/blob/0.293/presto-client/src/main/java/com/facebook/presto/client/StatementClientV1.java
- Memory worker lookup: https://github.com/prestodb/presto/blob/0.293/presto-memory/src/main/java/com/facebook/presto/plugin/memory/MemoryMetadata.java
- Image layout/heap: https://github.com/prestodb/presto/tree/0.293/docker

The tier first executed three stored goldens with its own fixture and a
one-or-two text parameter reader (#42, #43; notes in
[presto-container-notes.md](presto-container-notes.md),
[presto-parameters.md](presto-parameters.md) and [presto-time.md](presto-time.md)).
Those cases and `scripts/engines/presto-fixture.sql` are removed: the manifest's
forty cases include a grouped sum, text binding and the Tokyo day range with its
millisecond boundaries (#71). This closes #61. Open: transfer B0-PRESTO-003 out
of `seed-baseline-specs` in a product change once this tier has passed on GitHub.
