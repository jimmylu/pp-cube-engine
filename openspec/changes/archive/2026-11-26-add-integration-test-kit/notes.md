# Notes

## Done

- `ppcube-itest` with eight commands; the guide in Chinese; the benchmark as data; two Rust tests.
- Track A ran on all three engines; the guide's chapters that need no cluster were walked through.

## Evidence

All local, on real containers, service built from this checkout:

| Engine | Built-in suite |
| --- | --- |
| Presto 0.296 | 15 passed, 2 refused as expected, of 17 |
| Doris 2.1.8 | 16 passed, 1 refused as expected |
| Druid 26.0.0 | 15 passed, 2 refused as expected |

- Also over TLS with a read token, on the example's stack: `env: PASSED` with 9 of 9 tables, suite 15 passed and 2 refused.
- `load`, a second `load` that changes nothing, `load --dry-run`, `clean` without and with `--yes`: on all three engines.
- Chapter 6 (row-level security): `East 4, North 5`; under the rule `East 4`; with an `or` in the query still `East 4`, the rule a top-level `AND`.
- Chapter 7 (lifecycle) on a local service with metrics: a second version appears within the refresh period and the first stays; a version with a broken cube file is rejected and the others are served; removing a manifest retires a version; editing a loaded version is not seen.
- Chapter 10: `no SQL (QUERY)` twice with exit status 3; `NOT_FOUND`, `UNAUTHENTICATED` and `UNAVAILABLE` with exit status 2.
- The kit's unit tests: 14 pass. Not run in CI.
- Not verified: chapters 1, 2 and 8 of the guide (a release, a cluster of three replicas, upgrade and rollback). The guide marks them.

## Learned

- **A probe that asks a query cannot see Druid's NULL mode.** `'' IS NULL` answered "sql-compatible" on a Druid started in legacy mode. The mode is now read from the engine's own properties. This was found only because a legacy Druid was started to check the probe.
- A broken cube file is rejected with reason `files`, not `model`; the guide said `model` until the walk-through.
- The suite's hand-derived answer for year to date by quarter, added for Doris and Druid, was right on both at the first run; the Rust test had already confirmed which dialects compile it.
- A fresh Presto answers a version query before it can create a table ("No nodes available"); the first `load` after start failed and the second passed. A real cluster is not freshly started; the guide does not mention it.
- With `http_proxy` set in the shell, gRPC may try the proxy for the service's address. The guide names `no_proxy`.
- One image build of the example failed without output and passed when repeated; not reproduced, not explained.

## Open

- The owner's test is the point of this change: its findings come next.
- CI for the kit's tests.
- On a real Presto the benchmark needs a catalog that accepts `CREATE TABLE` and `INSERT`; it was run on the memory connector only. `load --dry-run` is the way round a catalog that does not.
- On a real Doris the replica count of new tables is a setting of the kit; three was not tried, the container has one backend.
