# Java client

A client of the pp-cube-engine compile service for Java 11 and later: a Cube query
in, SQL and its parameters out. It is the generated gRPC stubs of the service's
contract and a small typed layer over them. It executes nothing.

```java
try (CindervaleClient kernel = CindervaleClient.to("pp-cube-engine.internal:50051")
    .trust(Paths.get("/etc/portal/pp-cube-engine-ca.pem"))   // omit to trust the system's authorities
    .token(readToken)
    .build()) {

  CompiledQuery compiled = kernel.compile(
      Compilation.of("property/2026-11", "presto", queryJson)
          .timezone("Asia/Shanghai")
          .securityFilter(regionFilterJson)               // row-level rules of the caller's user
          .requestId(requestId)
          .build());

  try (PreparedStatement statement = warehouse.prepareStatement(compiled.sql())) {
    compiled.bind(statement);                             // never splice a value into the SQL
    try (ResultSet rows = statement.executeQuery()) { … }
  }
}
```

## What you get back

| | |
| --- | --- |
| `sql()` | One statement with `?` placeholders. Run it unchanged, on the engine of the dialect asked for |
| `parameters()`, `bind(statement)` | One value per placeholder, in order: `String`, `Double`, `Boolean` or `null` |
| `sessionProperties()` | Engine settings the SQL needs. Apply every one before executing |
| `columns()` | The result's columns: alias, the model member it stands for, its type, its granularity |
| `fingerprint()` | A key for caching. It covers the model content, query, dialect, timezone, security filters and kernel version, so it changes with every kernel version |
| `modelDigest()`, `kernelVersion()` | What produced the SQL |

## When there is no SQL

- `CompileRefusedException`: the service answered, without SQL. `kind()` says
  whose it is to fix: `MODEL`, `QUERY`, `UNSUPPORTED` (do not retry) or
  `INTERNAL`. The message names members and rules, never SQL or tables.
- `io.grpc.StatusRuntimeException`: the call failed. `NOT_FOUND` is an unknown
  model version, `UNAUTHENTICATED` a missing or wrong token,
  `RESOURCE_EXHAUSTED` a full service (back off), `UNAVAILABLE` and
  `DEADLINE_EXCEEDED` are worth one retry.

## Rules a caller keeps

- **Security filters are the caller's decision.** The kernel applies what it is
  given, where the query cannot widen it. Derive them from who the user is;
  never accept them from the user. The read token is therefore a trusted
  credential: it belongs to your back end, not to a browser.
- **One client, shared.** It is thread-safe and holds one connection. Several
  replicas behind a plain TCP balancer are spread by connection, not by request;
  see [deploying](../../docs/deployment.md).
- **Encrypted, with a token**, unless the service is on the same host:
  `plaintext()` is for loopback, and a token is never sent over it.

## Building

```sh
mvn -f clients/java/pom.xml install
```

The stubs are generated from `crates/pp-cube-service/proto/engine.proto` of
this checkout; there is no copy of the contract here, and the artifact carries
the kernel's version. It is not published to a repository yet: install it
locally, or build it in your own pipeline. `GetMeta` returns the generated
`GetMetaResponse`; the other operations of the contract are reachable through
the generated `CubeEngineGrpc` stubs.

A complete, runnable use is in
[examples/insurance-analytics](../../examples/insurance-analytics).
