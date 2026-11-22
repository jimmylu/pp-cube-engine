# Presto parameter smoke

Branch: harness/presto-parameters. Predecessor #42 merged; its final head passed
local CI, real local Docker, remote CI and Opus review (evidence in that PR).

## Design and scope

The next B0 execution gap is parameter binding. Keep this harness-only: extend the
existing HTTP verifier and disposable runner with one checked-in text equality
golden. Do not change product code, goldens, vendor, dependencies or baseline.

Use Presto 0.293's prepared-statement header, encoded as its official Java client
does, and EXECUTE USING with an escaped text literal. Avoid placeholder replacement.
The fixture uses C1/C2 channel values to match the stored parameter and verify
exclusion of the second group. Expected results remain independently chosen.

Alternatives: expanding temporal cases first leaves binding unverified; creating a
live compiler executable would require a separate product change. A generic Rust
Debug parameter parser is not warranted. Support exactly one Text in the checked-in
snapshot shape and reject other types/counts/escapes. No full B0/T2 acceptance claim.

## Tasks

- [x] Confirm #42 merge and start an isolated harness branch from current main.
- [x] Verify prepared-header encoding against pinned official client source.
- [x] Add one-Text snapshot parsing and prepared execution without SQL substitution.
- [x] Extend fixture and both entry points to check aggregate and filtered results.
- [x] Regress protocol quoting/encoding, wrong results and unsupported shapes.
- [ ] Obtain final-head local CI and real local Docker smoke evidence.
- [ ] Obtain remote t0/engine success and cross-vendor review; deliver Ready.

## Handoff

Offline Presto tests passed (20 tests). Binding protocol test first failed because
the client had no parameters argument; the implementation made it pass. Real local
Docker verification is running. Evidence binds to the eventual SHA in PR comments;
do not create evidence-only commits. Owner approves and merges.
