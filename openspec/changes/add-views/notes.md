# Notes

## 2026-11-24: investigation, design and implementation

The owner asked to continue with the next pull request after #85 merged.
Views, the rest of the eighth unit of the route. Implemented with Claude Code
(Anthropic).

Established first (design.md has the table): eighteen queries on a view over
four cubes were put to Cube Core 1.8.28 on Presto and to the kernel. What both
compile is byte-identical SQL with equal parameters, on the first run.

Done:
- Model: `views`, validated to a narrow form; a join-path reference form that
  only expanded views use; `Model::expanded` and `Model::underlying`.
- Bridge: view cubes and their members are flagged for the planner, with the
  join paths of more than one cube; references compile to the planner's paths.
- The kernel's own checks translate view members to the members behind them.
- A prepared model holds the expansion; the digest is of the model as written.
- Service: Compile on views; GetMeta lists views after the cubes.
- Five execution cases (70 in all), on the existing fixtures.

Evidence, on the working tree that became the implementation commit:
- Local, on demand: `check conformance: PASSED — Cube Core 1.8.28, 70 cases:
  SQL and parameters equal the kernel's but for Cube's default row limit, and
  results equal on Presto 0.293 (B0-PRESTO-002)`; `check doris-container:
  PASSED — Doris 4.1.3, 70 live-compiled manifest cases (B0-DORIS-002,
  B0-DORIS-010, B0-DORIS-011)`. Both passed on their first run.
  `engines-container` was not run locally.
- Existing SQL is unchanged: the manifest goldens gained five blocks and lost
  no line; no existing compile golden changed.
- CI_EVIDENCE
- Remote checks are recorded on the pull request. Cross-model review: not run.

Open:
- GetMeta has no flag that tells a view from a cube; it needs a wire field.
- Segments, members declared on a view, `alias`, and views as join-path hints
  are not supported.
- Refusals through a view name the member behind it, not the view's member.
- The eight units of the route are done with this change. `compareDateRange`
  and relative date ranges remain refused by design; Druid and other dialects
  were paused by the owner on 2026-11-23.

Learned: through a view the planner aggregates a measure of a cube joined
one_to_one apart, where the same query on the cubes is one SELECT; Cube Core
does the same. The planner's fixtures are a reliable description of how
Cube's compiler presents a model feature to the bridge.
