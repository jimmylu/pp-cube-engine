# Tasks

## 1. Manifest

- [x] 1.1 Add the manifest types and typed parameter mapping to `pp-cube-conformance` with a failing test for B0-EXEC-002 (uncompilable case, dialect without cases); verify with `cargo test -p pp-cube-conformance`.
- [x] 1.2 Add the `execution-cases` command and the conformance sales fixture; verify `cargo run -q -p pp-cube-conformance --bin execution-cases -- doris` prints valid JSON and an unknown dialect exits non-zero.

## 2. Doris temporal cases

- [x] 2.1 Move the thirteen temporal cases with fixtures and literal expectations into the manifest; add the B0-EXEC-001 test for byte-identical output, case names and scenario ids; verify with `cargo test -p pp-cube-conformance`.
- [x] 2.2 Make `real_doris_temporal_execution` and `doris_time_engine.py` consume the manifest; verify no golden changes with `cargo insta test --check` and that `scripts/check.sh fast` passes.
- [x] 2.3 Run the explicit temporal command on local Docker Doris and record the result line in notes.md; verify thirteen cases pass and the owned container is removed.

## 3. Integration

- [x] 3.1 Run `scripts/check.sh ci`, `scripts/check.sh engines-container` and `scripts/check.sh doris-container`; record each last line against the head SHA in the pull request.

## Workflow follow-up

- Sync and archive `execution-cases` before final review, in the same Draft PR.
- Harness pull request for issue #59 makes `doris-container` run the manifest in its one container.
- The Doris matrix change deletes the transitional test and verifier and transfers B0-DORIS-010, B0-DORIS-001 and B0-DORIS-002 from the seed.
