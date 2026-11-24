# Notes

## 2026-11-24: go-live review and route

The owner asked for a review against going live after #86 merged, gave the
deployment facts in review.md, added the WebAssembly validator as a
requirement to evaluate, and asked for the assessment to be put into the next
change. Written with Claude Code (Anthropic).

Done: review.md (gaps, the validator's evaluation, a proposed route, five
decisions); the proposal, tasks and spec delta of step 0, the
property-insurance scenario. No product code yet.

Established: `cargo build -p pp-cube-model --target wasm32-unknown-unknown
--release` succeeds at `e94bc19` with no change to the crate.

Not established, and stated as such in review.md: anything about Doris 2.1.8,
Presto 0.296 or Druid 26.0; the performance of executed SQL; the size of a
WebAssembly module.

Open: the owner's confirmation of the route and the five decisions; then
tasks 1.1 onward.
