# Druid dialect, B0

## Why

Druid 26.0 is in production and part of going live (owner, 2026-11-24; route
item G3). The kernel had no Druid dialect: no templates, no way to name it,
no cases.

## What Changes

- **A Druid dialect.** `druid` is a dialect a caller and a data source can
  name. Its templates are Cube's base templates with the overrides of Cube's
  Druid dialect at the pin. It compiles the B0 baseline, joined dimensions
  included. The seeded spec had assumed joins unsupported on Druid; on Druid
  26.0.0 they run.
- **Declared refusals.** A dialect's data file may list query features its
  engine cannot answer, from a fixed set. Druid lists six: SQL of more than
  one SELECT, ungrouped queries, rolling windows, time shifts, approximate
  distinct counts, declared granularities. Each is refused by name before or
  after planning; none surfaces as an internal error.
- **A query context every Druid result declares:**
  `useApproximateCountDistinct=false`. Druid estimates `COUNT(DISTINCT)`
  otherwise.
- **A Druid manifest** of the shared cases its dialect compiles, 52 of 85,
  with fixtures as SQL ingestions of inline rows and a reviewed list of the 33
  it refuses. The shared model is rewritten for Druid's one schema and one
  time column.
- Baseline level: B0 for Druid. New goldens only; nothing existing changes.

Non-goals: a Druid execution tier and a Cube Core comparison in the checks
(B0-DRUID-003, B0-DRUID-004), which are a harness change and follow; B1 on
Druid (route item G4): rolling windows, time shifts, approximate counts and
declared granularities need Druid templates, and multi-fact SQL is rejected
by the engine.

## Capabilities

### New Capabilities
- `dialect-druid`: compilation (B0-DRUID-001), declared refusals (B0-DRUID-002), query context (B0-DRUID-005).

### Modified Capabilities
- `execution-cases`: a manifest of part of the cases (B0-EXEC-006).

## Impact

Paths: `templates/druid.yaml`, `crates/pp-cube-dialect`, `crates/pp-cube-adapter`
(`windows.rs`, `lib.rs`), tests of `pp-cube-compile`, `crates/pp-cube-conformance`
(new `druid.rs`, `lib.rs`, goldens), `crates/pp-cube-service` (option text,
proto comment, README), `README.md`, `docs/guide.md`, `CHANGELOG.md`,
`templates/README.md`, the seed change, two specs and this change folder.
Callers: `DialectId` gains `Druid`, a break for code that matches on it.
