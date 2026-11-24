# Notes

## 2026-11-24: design and implementation

The owner asked for the next change after #93 merged, with two decisions for
route item G2: the store layout isolates business lines (up to three levels)
and asset types, leaving room for other semantic assets; model files follow
Cube's recommended naming. The question about S3 credentials was not
answered; it belongs to the S3 change. Implemented with Claude Code (Anthropic).

Done:
- `pp-cube-model::assemble`: a model from a directory of files, pure.
- `pp-cube-service::store`: the layout, the store trait, the directory store,
  the synchronizer with fixed rejection reasons, metrics and log lines.
- The process: `--model-store`, `--model-refresh-seconds`, a first pass before
  the address is announced, then a pass per interval.
- The service guide documents the layout and the operating rules.

Evidence, on the working tree that became the implementation commit:
- `cargo test -p pp-cube-service --test process a_replica`: the real binary
  loads a version from a store, serves it as soon as it announces its address,
  compiles a version published while it runs within the one-second interval,
  and stops offering one whose manifest is removed.
- No golden changed; no dialect or planner path changed, so the engine tiers
  were not run locally. Cross-model review: not run.
- CI_EVIDENCE

Open:
- The S3-protocol store: a second implementation of the trait. It needs an
  HTTP and request-signing stack in the service, a decision on credentials,
  and an S3-protocol server in the tiers.
- Replicas become consistent one refresh interval after a publication.
- Issue paths of a rejected model point into the assembled model, not a file.

Learned: the layout is a pure function of keys, which made every rule testable
without a file system; a completeness marker written last is the one
publication protocol a directory and a bucket share.
