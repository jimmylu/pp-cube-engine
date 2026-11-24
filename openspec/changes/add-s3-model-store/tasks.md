# Tasks

- [x] 1 Choose the dependencies against the policy: `object_store` fails `cargo deny`; `rusty-s3` with `ureq` passes. Verified with `cargo deny check`.
- [x] 2 The S3-protocol store: configuration from the environment, signed paginated listing and reading; test B1-STORE-004 against a stand-in server.
- [x] 3 Credentials from the environment or the instance role; test B1-STORE-005.
- [x] 4 Several stores in the process, numbered in the metrics; test B1-API-003 against the real binary.
- [x] 5 Log a rejection only when it changes; extend the B1-STORE-003 test.
- [x] 6 Run the binary against a real S3-protocol server that verifies signatures; record it in notes.md.
- [x] 7 Update the service guide, `docs/guide.md` and `CHANGELOG.md`.
- [ ] 8 Run `scripts/check.sh ci`; record the last line in notes.md.

## Workflow follow-up

- Sync and archive the delta before final review, in the same Draft PR.
- A harness change: an S3-protocol server in the tiers.
- Next on the route: G3, Druid 26.0.
