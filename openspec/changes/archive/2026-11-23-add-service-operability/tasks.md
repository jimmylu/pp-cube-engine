# Tasks

## 1. Library

- [x] 1.1 Add read and admin bearer tokens to the gRPC layer; test B0-API-016 over real TCP; verify `cargo test -p pp-cube-service --test operability`.
- [x] 1.2 Add the standard health service; test B0-API-017 including the drain flag.
- [x] 1.3 Add request metrics and JSON log events; test B0-API-018 including that no SQL, value, table or token appears.
- [x] 1.4 Add the compilation deadline, the version limit and the request size limit; test B0-API-019 (deadline as a unit test with an occupied worker).

## 2. Executable

- [x] 2.1 Add TLS, token files, the secure-listen rule, the metrics endpoint, limits options and SIGTERM drain; test B0-API-020 with a generated certificate; verify `cargo test -p pp-cube-service --test process`.
- [x] 2.2 Rewrite the service README as an operator's guide.

## 3. Integration

- [x] 3.1 Run `scripts/check.sh ci`; record the last line in notes.md.

## Workflow follow-up

- Sync and archive the delta before final review, in the same Draft PR.
- Next unit of the route: performance and release (planner context cache, benchmarks, container image, release process, aarch64, user documentation).
