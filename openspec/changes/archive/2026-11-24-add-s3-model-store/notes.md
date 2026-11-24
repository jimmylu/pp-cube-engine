# Notes

## 2026-11-24: design and implementation

The owner agreed to do the S3 store next, with credentials from the standard
environment variables and the instance role. Implemented with Claude Code
(Anthropic).

Done: `s3.rs` (configuration, signed list and read, instance-role
credentials); a repeatable `--model-store`; a `store` label on the store
metrics; rejections logged once.

Evidence, on the working tree that became the implementation commit:
- `cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok`.
  With `object_store` 0.14.2 instead it reported `advisories FAILED`.
- By hand, the debug binary against `versity/versitygw` on local Docker, the
  bucket filled by `amazon/aws-cli` with the insurance model split into nine
  cube files and two view files, plus one deliberately broken version:
  - a wrong secret: no address announced, `listing the bucket: status 403`;
  - the right keys: `property/motor/v1` loaded, `property/motor/v2` rejected
    with reason `files`, one version in the metrics;
  - a third version uploaded while it ran: two versions after the next pass.
- That run showed the rejection being logged at every pass, which is fixed
  and tested here.
- Not established: AWS itself, virtual-hosted addressing, and a real instance
  metadata service; these are covered by the stand-in only. MinIO could not
  be tried: its images were not pullable.
- No golden changed; the engine tiers were not run. Cross-model review: not run.
- `check fast: PASSED — 4 steps, 0 skipped, 9`; after archiving, `check ci: PASSED — `. A first run after archiving failed at
  `gitleaks`: two build artifacts under `target/` left by the `object_store`
  trial matched an AWS key pattern from that crate's documentation. They were
  untracked; removing them cleared it. Nothing in the repository matched.

Open:
- An S3-protocol server in the tiers (harness).
- Web identity credentials, if the deployment runs on Kubernetes on AWS.

Learned: check a dependency against the policy before designing around it.
A stand-in written by the author of the client proves the client's logic and
nothing about its signatures; the one real run found a defect the tests had not.
