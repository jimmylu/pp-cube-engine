# Tasks

## 1. Kernel

- [x] 1.1 Declare session properties in the dialect and carry them in the compile result; test B0-RESULT-003 and B0-PRESTO-017 for Presto and Doris with and without an offset; verify `cargo test -p pp-cube-compile`.
- [x] 1.2 Take the execution manifest's session from compile results and drop the hard-coded constant; verify the Presto and Doris manifests are unchanged and `cargo test -p pp-cube-conformance` passes with no golden change.

## 2. Service contract

- [x] 2.1 Rewrite the proto and gRPC mapping: typed parameters, structured used cubes, failure kinds, identity, session properties; test B0-API-010 over real TCP; verify `cargo test -p pp-cube-service`.
- [x] 2.2 Map data sources to dialects in the service core and the executable; test B0-API-011 and the startup failures of B0-API-009; verify the same command.
- [x] 2.3 Return typed, fuller metadata; test B0-API-012 including that no SQL or table name appears.

## 3. Records

- [x] 3.1 Transfer B0-PRESTO-003 from the seed; update the service README, the root README status and the design's proto listing; verify `openspec validate --all --strict` and traceability.
- [x] 3.2 Run `scripts/check.sh ci`; record the last line in notes.md.

## Workflow follow-up

- Sync and archive the deltas before final review, in the same Draft PR.
- Next unit of the route: model bundle lifecycle (multiple versions, verified digest, runtime publish and retire).
