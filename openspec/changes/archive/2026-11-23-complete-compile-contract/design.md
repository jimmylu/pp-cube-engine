# Design

## Context

See proposal.md. First unit of the productization route in `review.md`. The
owner allowed breaking changes: nothing consumes `ppcube.engine.v1` yet.

## Decisions

- **Rewrite v1 in place, no v2.** A second package would preserve a contract
  nobody uses. Field numbers are reassigned freely this once; from here on the
  proto evolves additively.
- **Session properties belong to the compile result.** The dialect declares
  them (`Dialect::session_properties(offset)`), the facade copies them into
  `Compiled`, the service sends them. They are not part of the fingerprint:
  they follow from the dialect and query, which already are. The conformance
  manifest unions them over its cases, so engine tiers set exactly what the
  kernel declares; its hard-coded Presto constant is removed. Alternative: keep
  the knowledge in documentation only, rejected because a caller that paginates
  on Presto would only learn of it from an engine error.
- **Typed parameters.** A `oneof` of text, number, boolean and null instead of
  JSON scalar strings: the caller binds without parsing, and a string that looks
  like a number stays a string. This resolves the seed's open question on wire
  parameter typing. Today every compiled parameter is text; the other arms
  exist because the planner's value type has them.
- **Data sources map to dialects in the service, not the model.** Which engine
  serves `warehouse` is a deployment fact; the same model can be deployed
  against different engines. `CubeService::new(data_sources)`; registration
  refuses a model with an unmapped data source (fail closed). The check on a
  compile uses the result's used cubes, so it also covers cubes that only a
  security filter joined in, at the cost of compiling before refusing. The
  request keeps its `dialect` field as an explicit assertion by the caller;
  deriving the dialect from the query was rejected because a caller would then
  receive SQL for an engine it did not ask about. This resolves the seed's open
  question on data source and dialect.
- **Identity on responses.** Kernel version and upstream pin are constants of
  the build; model version and digest come from the bundle. The digest stays
  the caller's opaque value; verifying it is part of the model lifecycle change.
- **Metadata.** Typed messages replace a JSON string. `meta` is passed as JSON
  text because it is free-form by contract. Joins to non-public cubes are
  omitted so discovery does not reveal a hidden cube's name. This reverses the
  earlier "names, types, titles and descriptions only" rule on purpose: a
  modelling interface needs format, segments and meta, and none of them is SQL.

## Risks / Trade-offs

- A mismatched dialect costs one wasted compilation → acceptable; it is a
  caller error path.
- `Compiled` and `CubeService::new` are source-breaking for Rust hosts → there
  are none outside this workspace.
- Presto's property is declared per query with an offset, while the manifest
  sends the union on every case → harmless, and the tier stays unchanged.

## Migration Plan

No deployed callers. Revert the pull request to restore the previous contract.
Evidence: `scripts/check.sh ci`, remote `t0`, `presto-smoke`, `doris-smoke`.
