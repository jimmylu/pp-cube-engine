# Design

## Context

See proposal.md. A store is `list` and `read` over keys; the layout, the
synchronizer and the rejection reasons are unchanged from the directory store.

## Decisions

- **`rusty-s3` and `ureq`, not `object_store`.** `object_store` was tried
  first: it fails the dependency policy on an unmaintained-crate advisory in
  its tree, pulls in a C cryptography library and adds about 490 lockfile
  lines. `rusty-s3` only signs; `ureq` is a blocking client, which fits a
  store that is called from a blocking pass. Together they pass the policy
  with about 170 lockfile lines and reuse the rustls and ring the service
  already has. The cost: the credential chain is ours, kept to two sources.
- **Two credential sources.** Environment keys, else the instance role by
  IMDSv2. Role credentials are asked for again after five minutes; the
  metadata service renews them well before expiry. Web identity (a token file
  exchanged with STS, as Kubernetes service accounts use on AWS) is not
  implemented and is refused when its variable is set, so that it is never
  mistaken for an instance role. Profiles and credential files are not read.
- **Path-style with a custom endpoint, bucket host name for AWS.** Enterprise
  S3-protocol storage is addressed by endpoint; path-style is what it accepts
  without DNS for each bucket.
- **Signed URLs, sixty seconds.** `rusty-s3` signs by query string, so no
  secret or signature is in a header to log by mistake; failures report the
  status only.
- **Stores are numbered.** Several stores share one catalog; a version name
  offered by two stores with different content is rejected in the second as a
  digest fault. Metrics carry the store's position rather than its location,
  which could name a bucket.
- **A rejection is logged when it changes.** The pass keeps the previous
  reasons; `log_lines` leaves out what is unchanged. The gauge still counts
  every rejected version.

## Evidence beyond the tests

The tests run against a stand-in server in the test process, which accepts
any signature. So the binary was also run by hand against Versity S3 Gateway
(`versity/versitygw`), a separate implementation that verifies signatures,
filled with the AWS CLI: see notes.md. MinIO's images could not be pulled
from either of its registries.

## Risks / Trade-offs

- A hand-kept credential chain → two sources, each tested; the rest refused.
- No S3-protocol server in CI → the stand-in covers the client's logic, the
  manual run its signatures; a tier is the follow-up.
- `http` endpoints sign but do not encrypt → said in the service guide.

## Migration Plan

Additive. Evidence: `scripts/check.sh ci`, remote `t0`, `image`, `aarch64`
and the engine checks.
