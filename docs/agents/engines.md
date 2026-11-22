# Presto execution smoke suite

## Scope

The harness executes three unchanged Presto 0.293 SQL goldens: grouped sum, one
string equality binding, and Tokyo day aggregation over an inclusive date range.
Columns and ordered rows are checked against independently chosen fixture results.
The kernel remains compile-only; no product, vendor, snapshot or dependency change
is required. This does not establish full B0/T2, other time granularities, DST,
other parameter types, other dialects or Cube Core conformance. T1 is not built.
The verifier reads stored compiler goldens; it does not compile live input.

## Run locally

With a working Docker daemon and amd64 support/emulation:

```sh
scripts/check.sh ci
scripts/check.sh engines-container
```

The container command pulls a digest-pinned Presto 0.293 amd64 image, provisions
one uniquely named container with a loopback-only ephemeral port and 2 GiB limit,
installs the memory catalog, checks the exact runtime build, probes Memory worker
execution, loads the fixture, runs all three goldens and removes the container.
Progress lines show each stage without publishing raw server responses.

For an already prepared disposable engine, install this catalog before startup:

```properties
connector.name=memory
```

Load `scripts/engines/presto-fixture.sql` with the Presto CLI, then run:

```sh
PRESTO_URL=http://127.0.0.1:8080 scripts/check.sh engines
```

Fixture table creation deliberately fails if the table already exists. Reset the
engine before repeating setup. Without PRESTO_URL, `engines` exits 2 (NOT RUN);
protocol/query/result failures exit 1. Only matching columns and rows pass.

## Assertions and parameter binding

The four-row fixture preserves aggregate results C1=30, C2=7 and the equality
result C1=30. Its UTC timestamps surround the existing Tokyo October range:

| UTC timestamp | Channel/amount | Tokyo day and inclusion |
| --- | --- | --- |
| 2026-09-30 14:59:59.999 | C1/10 | September 30; excluded |
| 2026-09-30 15:00:00.000 | C1/20 | October 1; included |
| 2026-10-31 14:59:59.999 | C2/7 | October 31; included |
| 2026-10-31 15:00:00.000 | C2/0 | November 1; excluded |

The temporal result must be October 1=20, October 31=7. The zero-valued outsider
keeps earlier totals unchanged but produces an extra November bucket if admitted.
Session timezone remains UTC; the golden SQL applies Asia/Tokyo explicitly.

Text binding follows the pinned Presto client's prepared-statement header and
`EXECUTE ... USING`. SQL retains its original `?` placeholders. Prepared SQL is
form-URL-encoded in `X-Presto-Prepared-Statement`; values become apostrophe-escaped
text literals in USING. Placeholder substitution is never used.

Only one or two Text parameters in the checked-in Rust Debug snapshot shape are
supported. Numeric, null, three or more parameters, mixed types and Rust-only
string escapes fail closed. This is not a general Debug parser. Offline protocol
regressions cover encoding, quoting, Unicode, parameter order and result failures.

## Bounds, cleanup and delivery

Each HTTP query has a 30-second POSIX wall-clock deadline including body reads,
503 retries, and best-effort cancellation. Redirects, cross-origin continuation,
query errors, changing columns and excessive rows fail. Public failures use fixed
allowlisted diagnostics rather than endpoints or server messages.

Docker commands have explicit timeouts. Version readiness uses runtime.nodes,
because this build rejects version(). The pinned image reports 0.293-2f48ff6.
Memory worker readiness uses a bounded CTAS probe; only known no-nodes errors are
retried. Each attempt has a distinct table to isolate partial outcomes. The
successful probe is dropped and all remaining state disappears with the container.
Cleanup is attempted after any create attempt, even a timeout. Ordinary cleanup
failure fails the run. SIGKILL or shutdown can leave an owned cube-presto- container
requiring manual removal; pre-existing containers are never removed.

`scripts/dev deliver` runs local CI, real local Docker smoke, cross-vendor review
and remote t0 before Ready. Every repair round repeats both local checks. Engine
failure stops before review; remote success cannot replace local proof. T0 remains
Docker-free. The separate presto-engine CI runs the real suite on relevant PRs.
Its log/summary records the tested merge SHA and final smoke line. Evidence binds
to the PR head and recorded merge SHA in comments/Actions, not evidence-only commits.
The owner approves and merges.

## Sources and handoffs

- Protocol: https://github.com/prestodb/presto/blob/0.293/presto-docs/src/main/sphinx/develop/client-protocol.rst
- Header encoding: https://github.com/prestodb/presto/blob/0.293/presto-client/src/main/java/com/facebook/presto/client/StatementClientV1.java
- Memory worker lookup: https://github.com/prestodb/presto/blob/0.293/presto-memory/src/main/java/com/facebook/presto/plugin/memory/MemoryMetadata.java
- Image layout/heap: https://github.com/prestodb/presto/tree/0.293/docker
- Current design/tasks: [presto-time.md](presto-time.md).
- Historical handoffs: [presto-container-notes.md](presto-container-notes.md) and
  [presto-parameters.md](presto-parameters.md); their final evidence is in merged PRs #42/#43.
