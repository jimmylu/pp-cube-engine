# Tasks

## 1. Sources

- [x] 1.1 Add `--sources` to the emitter with model, data sources, query, timezone and security filters; test B0-EXEC-005; verify the plain Presto manifest is byte-identical to before and `cargo test -p pp-cube-conformance` passes.
- [x] 1.2 Compare by hand with Cube Core 1.8.28 and record the result in notes.md and `docs/guide.md`.

## 2. Packaging

- [x] 2.1 Pin the Dockerfile base images by digest; verify `scripts/check.sh image` passes locally.

## 3. Integration

- [x] 3.1 Run `scripts/check.sh ci`; record the last line in notes.md.

## Workflow follow-up

- Sync and archive the delta before final review, in the same Draft PR.
- Harness pull request: `scripts/check.sh conformance` and a `cube-core` job run the comparison on GitHub.
- Then transfer B0-PRESTO-002 from the seed, with the join work (#22, #23, #30).
