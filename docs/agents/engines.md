# Presto execution smoke test

## Scope and design

The harness executes two unchanged existing goldens on Presto 0.293: grouped sum
and equality on one string dimension. It checks columns and ordered rows from a
neutral three-row fixture. This covers basic aggregation and one text parameter;
it does not establish full B0/T2, temporal execution, other parameter types, other
dialects or Cube Core conformance. The Rust kernel remains compile-only.

The standard-library HTTP client bounds each query to 30 seconds, including body
reads, retries 503, rejects redirects and cross-origin continuation, and attempts
cancellation. Text binding follows Presto's prepared-statement request header and
`EXECUTE ... USING`: the golden SQL retains `?`. The prepared SQL is form-URL-encoded
in `X-Presto-Prepared-Statement`; the text value becomes an apostrophe-escaped SQL
literal in USING. This is the engine's binding protocol, not placeholder replacement.

Only the checked-in one-Text snapshot shape is supported. Snapshot parameters are
Rust Debug output, not a general serialization format: numeric, null, multiple
parameters and Rust-only string escapes fail closed. The protocol test includes
apostrophes, SQL-like punctuation and Unicode. Authenticated endpoints and arbitrary
session mutations remain outside this client.

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
The runner executes both goldens; the grouped-sum case requires empty parameters.
The equality case reads its single Text parameter without replacing the placeholder. Headers select memory/dw and UTC. The three
fixture rows yield `C1,30` then `C2,7`; the equality filter yields only `C1,30`.
The runner does not create, drop or alter tables. Without PRESTO_URL it exits 2
(NOT RUN); connection, protocol, SQL and comparison failures exit 1; only matching
results exit 0. Queries have a 30-second request budget and best-effort cancellation.

A successful command covers two Presto smoke cases only. Record its last line with
the Git SHA and engine version in PR/CI artifacts. Do not publish endpoint names or
server responses. No engine test runs implicitly in T0; offline protocol tests run
there and cannot count as T2 evidence.

## Tasks and handoff

- [x] Design an isolated harness slice using an unchanged existing golden.
- [x] Add the HTTP runner, fixture and check entry point.
- [x] Cover successful pagination/retry and failure paths with offline tests.
- [x] Keep missing engine configuration visibly NOT RUN.
- [ ] Establish exact-head local CI, cross-vendor review and remote t0 via delivery.

The prepared-engine command was introduced in #41. At that point the local Docker
daemon was unavailable, so real local execution was not run. The container entry
point below now provisions the engine, and local real execution has passed. Final
head-bound CI, engine and review evidence belongs in PR comments. One smoke case
does not establish full T1/T2 acceptance. Parameterized and temporal execution
cases remain separately scoped work. Owner reviews and merges.

## Sources

Protocol and memory setup follow the pinned Presto 0.293 official documentation:

- https://github.com/prestodb/presto/blob/0.293/presto-docs/src/main/sphinx/develop/client-protocol.rst
- https://github.com/prestodb/presto/blob/0.293/presto-docs/src/main/sphinx/connector/memory.rst

## Reproducible container entry point

`harness/presto-container` adds `scripts/check.sh engines-container`. On a Docker
host it pulls the Presto 0.293 linux/amd64 manifest pinned by digest, creates one
uniquely named container with a loopback-only ephemeral port and 2 GiB memory,
installs the memory catalog before startup, checks `SELECT DISTINCT node_version FROM system.runtime.nodes`, loads the
existing fixture and calls the existing golden verifier. It always attempts to
remove its unique container after a creation attempt, including on readiness, setup or assertion
failure. It never removes a pre-existing container. Docker commands have explicit
timeouts; the readiness loop is bounded. Ordinary Python exceptions are sanitized
at the CLI boundary. SIGKILL or machine shutdown can prevent cleanup; abandoned
containers can be identified by their `cube-presto-` prefix.

The separate `presto-engine` workflow runs this command for relevant pull requests
and manual dispatches, with read-only repository permissions and no credentials
passed to the engine. The Actions job log and summary record the SHA and final
smoke line. A pull-request run tests GitHub's merge commit; use its recorded SHA
alongside the PR head and t0 evidence. It does not replace t0 or expand the smoke
case into full T2 acceptance. No kernel, snapshot or dependency change is included.

Design choice: one standard-library lifecycle runner reuses the HTTP verifier;
manual setup remains available, while multi-engine Compose orchestration is deferred.
The fixed amd64 manifest also runs on arm hosts only when Docker emulation is
available. Source for image layout, CLI path and memory sizing:
https://github.com/prestodb/presto/tree/0.293/docker

Container tasks are implemented: pinned provisioning, readiness/version check,
fixture setup, golden verification, cleanup, offline regressions and CI. Local and remote
real engine execution have both passed. The local Docker daemon is now available;
the first local run completed fixture loading, golden verification and cleanup.
Current delivery evidence binds to the head SHA in PR comments and Actions.

Two readiness defects were diagnosed with controlled remote evidence: this engine
rejects `version()` as unregistered, so the probe reads `node_version` from
`system.runtime.nodes`; the pinned manifest reports `0.293-2f48ff6`, not `0.293`.
The runner now requires that exact build identity. A lifecycle regression reproduces
the real version string and rejects other versions before fixture setup. Fixed
stage and allowlisted error diagnostics distinguish subsequent execution failures.
Sources: https://github.com/prestodb/presto/blob/0.293/presto-docs/src/main/sphinx/connector/system.rst

See [presto-container-notes.md](presto-container-notes.md) for the repair handoff.
Owner reviews and merges after final evidence and cross-vendor delivery.

## Local delivery prerequisite

The owner requires local Docker execution before review and merge. The delivery
controller runs `ci`, then `engines-container`, then cross-vendor review and the
remote t0 gate. Every repair round repeats both local checks. Docker unavailability
or a failing engine smoke stops delivery; remote success cannot replace local proof.
Run `scripts/check.sh engines-container` directly to reproduce. Fixed progress lines
show image pull, startup, readiness, fixture, SQL execution and cleanup without raw
server output. The ordinary `ci` command remains Docker-free for the separate T0 job.
This is a real-engine integration smoke suite (two stored goldens), not a new
model-to-execution end-to-end suite.

Memory readiness must exercise execution: the version query can pass before worker
nodes are advertised. Before loading the fixture, the runner performs a bounded
Memory CTAS probe (`SELECT 1`), retrying only the fixed no-nodes errors. Every attempt
uses a different probe table to isolate partial CTAS outcomes; the successful probe
is dropped and all remaining state disappears with the disposable container.
Other errors and timeouts fail immediately. This preserves the fixture and golden
assertions. Source for Memory creation and worker lookup:
https://github.com/prestodb/presto/blob/0.293/presto-memory/src/main/java/com/facebook/presto/plugin/memory/MemoryMetadata.java

## Parameter-binding design and sources

The fixture now uses C1/C2 channel values so the unchanged string-equality golden's
`Text("C1")` selects two rows totaling 30 and excludes a row totaling 7. Both direct
and container commands execute the same suite. Existing aggregate semantics and
SQL snapshots are unchanged. See [presto-parameters.md](presto-parameters.md).

The prepared-header form encoding follows the pinned official client implementation:
https://github.com/prestodb/presto/blob/0.293/presto-client/src/main/java/com/facebook/presto/client/StatementClientV1.java
Protocol documentation:
https://github.com/prestodb/presto/blob/0.293/presto-docs/src/main/sphinx/develop/client-protocol.rst
