# Notes

## 2026-11-25: measurement and rework

The owner asked why delivery took so long and agreed to the plan: one commit
for implementation and archive, no separate local `ci` before `deliver`, no
routine local engine tiers, and this rework before Druid. Implemented with
Claude Code (Anthropic).

Done: the determinism replay keeps four workers per test binary.

Evidence, on the working tree that became the commit:
- `cargo test -p pp-cube-compile`: all pass; the eleven replaying binaries
  take 4 to 6 seconds each, about 57 seconds together, against about 536
  before. No worker process remained afterwards.
- Mutation: a worker answering differently once failed the replay in all four
  processes of the binary tried; reverted.
- `check ci` and the remote checks are recorded on the pull request by
  `scripts/dev deliver`. Cross-model review: not run.

Open: helpers repeated across test files; a faster runner in the harness.

Learned: measure before reworking. One helper was nine of a check's twelve
minutes, and delivery ran the suite four times a pull request.
