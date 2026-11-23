# Notes

## 2026-11-23: design and implementation

The owner confirmed the productization route (review.md) and that breaking the
wire contract is allowed because no caller exists. Implemented with Claude Code
(Anthropic). First unit of the route.

Done:
- `Compiled.session_properties`, declared by the dialect; Presto declares
  `offset_clause_enabled=true` only for a query with an offset. The execution
  manifest takes its session from compile results; both manifests are unchanged.
- `ppcube.engine.v1` rewritten: typed parameters, structured used cubes with
  data source, failure-kind enum, identity on Compile and GetMeta, typed metadata.
- `CubeService::new(data_sources)`: unmapped data sources refuse registration;
  a dialect that does not serve a cube's data source is a query failure without
  SQL. Executable option `--data-source name=dialect`.
- GetMeta: data source, segments, format, primary keys, meta, joins to public cubes.
- B0-PRESTO-003 transferred from the seed; READMEs and the design note updated.

Evidence: `check ci: PASSED — 15 steps, 0 skipped, 216s @ 894fecd+dirty` on the
working tree that became the implementation commit. No golden changed. Remote
checks are recorded on the pull request by delivery. Cross-model review: not run.

Open (route in review.md): model bundle lifecycle next. Not done here and worth
knowing: the dialect check compiles before it refuses; `druid` is still answered
as an unsupported failure; the model digest is still opaque and unverified.

Learned: every parameter the planner emits today is text, including numbers and
booleans, so the number, boolean and null arms of the wire type are reachable
only through the planner's value type and are untested end to end.
