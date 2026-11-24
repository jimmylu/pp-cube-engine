# Notes

## 2026-11-24: Doris 2.1.8, first part of G1

The owner asked for the next step after #91 merged: route item G1, production
engine versions. Implemented with Claude Code (Anthropic).

Established by hand, outside the tiers:
- Presto 0.296 (the published image, build `0.296-8661b73`): all 85 cases pass
  with no change. Its pin is a harness change and is not in this pull request.
- Doris 2.1.8 has no single-container image, only `fe-2.1.8` and `be-2.1.8`.
  Run as two containers on one network they report `doris-2.1.8-rc01`.
  84 of 85 cases passed; design.md has what failed and what else was tried.

Done: quarters as months in the two Doris templates; B1-DORIS-005.

Evidence, on the working tree that became the implementation commit:
- Doris 2.1.8, by hand, every case one by one: 85 of 85 pass.
- `check doris-container: PASSED — Doris 4.1.3, 85 live-compiled manifest
  cases (B0-DORIS-002, B0-DORIS-010, B0-DORIS-011)`.
- Goldens: two lines of one compile golden and one line of the Doris manifest
  golden change, each a quarter written as months.
- Not run: `conformance` and `engines-container`; no Presto path changes.
  Cross-model review: not run.
- CI_EVIDENCE

Open:
- The harness change: both tiers on the production versions. The Doris tier
  has to provision a frontend and a backend instead of one container.
- Druid 26.0 is G3.

Learned: an execution case that passes on one engine version says little
about another two major versions away; trying each function the dialect can
write found a defect no case reached.
