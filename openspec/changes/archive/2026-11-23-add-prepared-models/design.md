# Design

## Context

See proposal.md. Fourth unit of the productization route. The design document
(6.5) planned a per-thread planner context cache; nothing had been measured.

## Decisions

- **Measure first.** `compile-bench` times validation, digest and compilation
  separately on a model padded to a chosen size. The result redirected the
  work: the planner was never the large cost, the per-call validation was.
- **Prepare the model, not the planner.** `PreparedModel` holds the shared model
  and its 32-byte content digest, and can only be built by validating. The
  fingerprint takes that digest instead of recomputing it, so its bytes do not
  change. It is `Send + Sync`; nothing planner-side is cached, so the planner's
  thread-safety limits are untouched. The per-thread cache was not built: after
  preparing, a compilation costs 0.3 to 2 ms and scales across threads, and a
  cache of `Rc` planner state would add invalidation and memory questions for
  an unmeasured gain.
- **`compile` stays.** It prepares and compiles in one call, so existing
  callers and all goldens are unaffected, and its cost is documented.
- **The service prepares at publish.** A version maps to its bundle and its
  prepared model under one lock; a compilation clones the handle and releases
  the lock before planning. Publishing validates twice (bundle and prepared
  model); publishing is rare and this keeps both types honest on their own.
- **Benchmark as a binary in the conformance crate.** It reuses the shared
  cases and needs no benchmark dependency. It is not run by any check: timings
  on shared runners are noise.
- **Image.** A two-stage Dockerfile: the pinned Rust image builds with
  `--locked` from the context's `vendor/`, and a distroless `cc` image runs the
  binary as non-root with no shell. `.dockerignore` admits only what the build
  reads. The image is built by hand until a workflow builds it.

## Risks / Trade-offs

- What remains grows with the model (about 10 µs per cube) and is unprofiled.
- Figures come from one laptop; they show ratios, not a service-level target.
- The image build is not exercised by CI yet, and its base images are tags, not
  digests → the release pull request pins and builds them.

## Migration Plan

Additive. Revert to return to per-call validation. Evidence: `scripts/check.sh
ci`, remote `t0`, `presto-smoke`, `doris-smoke`; benchmark and image build run
by hand and recorded in notes.md.
