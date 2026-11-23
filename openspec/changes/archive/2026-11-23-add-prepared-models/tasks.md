# Tasks

## 1. Measure and fix

- [x] 1.1 Add `compile-bench` and the padded-model helpers; verify it runs at 4, 50 and 200 cubes and its direct and prepared results are equal.
- [x] 1.2 Add `PreparedModel` with the B0-RESULT-004 test; verify `cargo test -p pp-cube-compile` passes with no golden change.
- [x] 1.3 Prepare versions at publish in the service; verify `cargo test -p pp-cube-service`.
- [x] 1.4 Record method and figures in `docs/performance.md`.

## 2. Package and document

- [x] 2.1 Add the Dockerfile, ignore file and an example client; verify the image builds locally and the container serves a compilation over TLS with a token.
- [x] 2.2 Write `docs/guide.md` and link it from the README.

## 3. Integration

- [x] 3.1 Run `scripts/check.sh ci`; record the last line in notes.md.

## Workflow follow-up

- Sync and archive the delta before final review, in the same Draft PR.
- Harness pull request: build the image and an aarch64 target in CI, pin base images by digest, define versioning and the release workflow.
