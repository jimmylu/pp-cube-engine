# Presto execution smoke test

## Scope and design

Branch `harness/presto-execution` adds the first T2 entry point. Execute the existing
`aggregation__one_sum_grouped_by_one_dimension.snap` SQL unchanged against a
prepared Presto instance; assert column names and ordered rows from independently
chosen fixture data. This catches SQL syntax, catalog binding and grouped-sum
execution mistakes. It does not establish full B0, other dialects, parameter binding,
timezone, security or Cube Core conformance. The Rust kernel remains compile-only.
No product code, snapshot or dependency changes belong in this PR.

A full container orchestrator and Cube Core comparison are larger independent
steps. This slice uses Python's standard library HTTP client and an explicitly
provided disposable engine on POSIX (Linux/macOS), whose signal timer enforces the
wall-clock bound. HTTP pagination, 503 retries, a total request deadline,
query errors, cancellation and result mismatches have offline HTTP regressions.
Review repairs cover slow-body wall deadlines, sanitized HTTP protocol failures,
cancellation preserving the original error, and redirects with Location headers.
The redirect assertion requires the explicit refusal error and exactly one request;
a failed request after following a redirect cannot pass it.
The slow-body test streams bytes within the socket timeout so it tests the total
deadline rather than racing the socket timeout on Python 3.9.
Redirects and cross-origin continuation URLs fail. Session-mutating SQL and
authenticated endpoints are outside this small client's scope.

## Run

Use a disposable Presto 0.293 instance with `etc/catalog/memory.properties`:

```properties
connector.name=memory
```

Run `scripts/engines/presto-fixture.sql` using the Presto CLI against that instance.
The fixture creates `memory.dw.sales_dtl`; it intentionally fails if the table
already exists. Reset the disposable instance before repeating setup. With its
HTTP origin in `PRESTO_URL`, run:

```sh
scripts/check.sh ci
scripts/check.sh engines
```

`ci` first establishes that the checked-in golden agrees with the current compiler.
The runner executes that golden, requiring explicitly empty parameters. It does
not interpolate values or rewrite SQL. Headers select memory/dw and UTC. The three
fixture rows yield `web,30` then `store,7`, including the golden's descending order.
The runner does not create, drop or alter tables. Without PRESTO_URL it exits 2
(NOT RUN); connection, protocol, SQL and comparison failures exit 1; only matching
results exit 0. Queries have a 30-second request budget and best-effort cancellation.

A successful command covers one Presto smoke case only. Record its last line with
the Git SHA and engine version in PR/CI artifacts. Do not publish endpoint names or
server responses. No engine test runs implicitly in T0; offline protocol tests run
there and cannot count as T2 evidence.

## Tasks and handoff

- [x] Design an isolated harness slice using an unchanged existing golden.
- [x] Add the HTTP runner, fixture and check entry point.
- [x] Cover successful pagination/retry and failure paths with offline tests.
- [x] Keep missing engine configuration visibly NOT RUN.
- [ ] Establish exact-head local CI, cross-vendor review and remote t0 via delivery.

Implementation is ready for checkpoint; delivery evidence belongs in PR comments,
without evidence-only commits. Docker CLI is installed in this workspace but the
daemon is unavailable, so real Presto execution is NOT RUN. No T1/T2 acceptance is
claimed. Owner reviews and merges; no product change or baseline expansion is needed.
Next: run this smoke case on a prepared engine, then add parameterized and temporal
cases and reproducible engine provisioning in separately scoped changes.

## Sources

Protocol and memory setup follow the pinned Presto 0.293 official documentation:

- https://github.com/prestodb/presto/blob/0.293/presto-docs/src/main/sphinx/develop/client-protocol.rst
- https://github.com/prestodb/presto/blob/0.293/presto-docs/src/main/sphinx/connector/memory.rst
