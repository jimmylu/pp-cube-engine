# Sources for the Cube Core comparison

## Why

Nothing had ever compared the kernel with Cube itself; goldens only showed that
output is stable. A comparison needs to ask Cube Core the same question, and
the execution manifest carried only the compiled SQL.

A first comparison was run by hand before writing this: Cube Core 1.8.28, the
release tagged at the pinned upstream commit, returned SQL and parameters equal
to the kernel's for all forty Presto cases, except that Cube's API adds
`LIMIT 10000` to a query without a limit.

## What Changes

- `execution-cases <dialect> --sources` emits the manifest with the model text,
  its data sources, and each case's Cube query, timezone and security filters.
  The plain manifest is byte-identical, so existing tiers are unaffected.
- `docs/guide.md` records how the kernel compares with Cube Core and the one
  difference found.
- The Dockerfile's base images are pinned by digest.
- Baseline level: B0. No compile output or golden changes.

Non-goals: the tier that runs Cube Core (a harness path; the next pull
request, already drafted and run locally); B0-PRESTO-002, which stays proposed
in the seed until that tier passes on GitHub; adopting Cube's default row limit;
Doris (Cube has no Doris dialect); #22, #23 and #30, the next product change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities
- `execution-cases`: adds the sources manifest (B0-EXEC-005).

## Impact

Paths: `crates/pp-cube-conformance` (emitter, renderer, test), `Dockerfile`,
`docs/guide.md`, the spec and this change folder.
