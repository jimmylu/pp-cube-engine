# Notes

## 2026-11-22: proposal

Owner agreed in conversation to create this change and its Draft PR after a
progress review; review.md holds the gaps and route. Prepared with Claude Code
(Anthropic). Proposal, spec delta, design and tasks are written; no product code
or harness path is changed and no task is started.

Open: all tasks. No owner decision blocks implementation. The order of Druid
compilation and Presto T1 in review.md is a recommendation the owner may change.

Learned: the opt-in temporal verifier starts its own container, so calling it
from the standard Doris tier would start two engines; the manifest lets the
tier run every case in one.
