# Tasks

## 1. Contract

- [x] 1.1 Assemble a model from a directory of files, purely; test B1-MODEL-004; verify `cargo test -p pp-cube-model --test assemble`.

## 2. Store

- [x] 2.1 The layout as a pure function of keys; test B1-STORE-001.
- [x] 2.2 The store trait, the directory store, offering and rejecting versions with fixed reasons, metrics and log lines; test B1-STORE-002.
- [x] 2.3 Later passes: add, retire, leave other versions, survive an unreadable store; test B1-STORE-003.
- [x] 2.4 `--model-store` and `--model-refresh-seconds` in the process; test B1-API-002 against the real binary.

## 3. Evidence

- [x] 3.1 Update the service guide, `docs/guide.md` and `CHANGELOG.md`, with the engine versions now in the tiers.
- [ ] 3.2 Run `scripts/check.sh ci`; record the last line in notes.md.

## Workflow follow-up

- Sync and archive the deltas before final review, in the same Draft PR.
- Next: the S3-protocol store, with an S3-protocol server in the tiers.
