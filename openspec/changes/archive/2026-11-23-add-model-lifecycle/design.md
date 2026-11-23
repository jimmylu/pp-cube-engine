# Design

## Context

See proposal.md. Second unit of the productization route
(`archive/2026-11-23-complete-compile-contract/review.md`). The owner asked to
follow the recommended plan; the one open question was how versions arrive at
run time.

## Decisions

- **Explicit operations, not a watched directory.** PublishModel, RetireModel
  and ListModels on the existing service. A publish is one validated,
  acknowledged request with a definite outcome; a directory watcher has partial
  writes, platform-specific events and no place to report a rejected model.
  Startup still takes model files, so a restart restores what the host was
  started with. Persistence and multi-instance fan-out stay with the host for
  now: publishing is idempotent, so a host can repeat it to every instance.
- **Immutable versions, idempotent publish.** Same version and same digest is
  success with `created: false`; a different digest is ALREADY_EXISTS. The
  catalog itself keeps rejecting every duplicate (B0-MODEL-003); the service
  compares digests before it registers. To change content, publish a new
  version and retire the old one.
- **Retire is removal from the catalog only.** A compilation holds an
  `Arc<ModelBundle>` from the moment it selects its version, so retirement
  cannot fail work in progress and needs no draining. A retired name may be
  published again; the digest in each response says which content answered.
- **Shared-reference mutation.** `CubeService` holds `RwLock<ModelCatalog>`;
  readers clone the handle and drop the lock before planning, so a publish never
  waits for a compilation. Every write is a single insert or remove, so a
  poisoned lock is recovered rather than propagated.
- **Digest of typed content.** SHA-256 over the canonical JSON of the loaded
  model, the same bytes the fingerprint already hashes, prefixed `sha256:`.
  Hashing the uploaded bytes was rejected: reformatting a file would look like a
  new model. This resolves the seed's open question on who verifies the digest.
  `expected_digest` is optional because a publisher that just validated the
  same bytes gains nothing from repeating it, while a pipeline that built the
  model elsewhere can pin it.
- **Management off by default.** Whoever can publish decides the SQL every
  caller receives. Until the service has authentication, the only guard is that
  the host must opt in (`with_management(true)`, `--allow-publish`) on a socket
  it controls. ListModels is gated with the other two so a caller cannot
  enumerate versions it was not told about.
- **Rejected models are a response, not a status.** An invalid model returns
  its located issues, as ValidateModel does. Digest mismatch, unmapped data
  source, blank version and conflicting content are statuses.

## Risks / Trade-offs

- Published versions are lost on restart → documented; the host republishes.
  Durable storage belongs with the deployment change.
- Management without authentication → off by default and loopback-only today.
- Unbounded number of versions → retiring is the host's job; limits come with
  the operability change.

## Migration Plan

No deployed callers. Revert to return to one startup version. Evidence:
`scripts/check.sh ci`, remote `t0`, `presto-smoke`, `doris-smoke`.
