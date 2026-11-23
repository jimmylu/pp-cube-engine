# Tasks

## 1. Catalog and digest

- [x] 1.1 Add ordered listing and retirement to the catalog with the B0-MODEL-009 test; verify `cargo test -p pp-cube-model --test bundles`.
- [x] 1.2 Export the content digest and make the service compute it; test B0-API-014 for formatting independence and expected-digest mismatch; verify `cargo test -p pp-cube-service --test operations`.

## 2. Lifecycle

- [x] 2.1 Publish, list and retire through a shared reference with immutable, idempotent versions; test B0-API-013 including concurrent compilation; verify the same command.
- [x] 2.2 Add ListModels, PublishModel and RetireModel to the proto and gRPC layer, off unless enabled; test B0-API-015 and the lifecycle over real TCP; verify `cargo test -p pp-cube-service --test grpc`.
- [x] 2.3 Give the executable repeatable `--model version=path` and `--allow-publish`; update B0-API-008 and B0-API-009 tests and the service README; verify `cargo test -p pp-cube-service --test process`.

## 3. Integration

- [x] 3.1 Run `scripts/check.sh ci`; record the last line in notes.md.

## Workflow follow-up

- Sync and archive the deltas before final review, in the same Draft PR.
- Next unit of the route: deployable and operable (TLS, authentication hook, health, logs, metrics, deadlines, limits).
