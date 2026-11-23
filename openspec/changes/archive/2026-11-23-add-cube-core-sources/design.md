# Design

## Context

See proposal.md. Fifth unit of the productization route, product half. The
comparison itself is a harness tier; this change gives it its input and records
what the first run found.

## What was established first, by hand

- The upstream pin is exactly the commit of the tag `v1.8.28`, so the image
  `cubejs/cube:v1.8.28` is the same Cube.
- In development mode Cube Core answers `GET /cubejs-api/v1/sql` without
  connecting to a database; it only needs a driver type per data source to pick
  a dialect. No engine is needed to obtain its SQL.
- Cube Core reads the conformance model file unchanged, as a Cube model.
- All forty Presto cases: SQL and parameters equal the kernel's, except
  `LIMIT 10000` appended when the query has no limit. With the drafted tier the
  Cube SQL also returned every case's expected rows on Presto 0.293.

## Decisions

- **A flag, not new fields in the plain manifest.** The engine tiers accept
  exactly the plain case fields and reject anything else; adding fields would
  break them before a harness change could follow. `--sources` leaves the plain
  output byte-identical. A test strips the added fields and compares.
- **The query as written.** The case's own JSON text is emitted, not a
  re-serialization of the loaded query, so Cube is asked what the case says.
- **Data sources in the manifest.** Cube needs a driver type for each; the
  harness should not parse model YAML to find them.
- **The default row limit is recorded, not adopted.** It is behaviour of Cube's
  API gateway, configurable there, and the kernel's contract says the caller
  owns such policy. Adopting it would change every golden without a limit.
- **Digest pins.** The image built in CI should not change under a moving tag.

## Risks / Trade-offs

- The comparison ran against Cube's default planner. Whether that is the Rust
  planner this kernel embeds or Cube's JavaScript one was not determined; either
  way it is what Cube Core 1.8.28 emits.
- Forty cases on one small model: agreement on B0, not a proof of equivalence.
- Until the tier is in CI the result above is local evidence only.

## Migration Plan

Additive. Evidence: `scripts/check.sh ci`, remote checks including `image`
(the Dockerfile changes).
