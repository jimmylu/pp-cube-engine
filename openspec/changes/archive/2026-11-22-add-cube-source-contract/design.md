# Design

## Context
Main includes merged Presto golden acceptance PR #37. Cube already has typed fields, strict serde loading and format-neutral validation. validation.rs requires name/data_source and exactly one nonempty sql_table/sql; model.rs keeps source exclusivity as validation rather than loading behavior.

## Goals / Non-Goals
Adopt one complete seed requirement with meaningful tests. Do not alter runtime semantics, add bundle identity or widen B0.

## Decisions
Use one source-selection test to exercise all four B0-MODEL-002 cases in both encodings, asserting complete (path, rule) reports. This satisfies the seed's matrix policy: each test naming an id covers every member it names. A second test covers every optional cube field with each valid source, compares YAML/JSON typed models and round trips, and rejects an unknown cube field under B0-MODEL-006. Use neutral inline fixtures; no upstream access is needed for already implemented model-shape rules.

Alternatives: adding a dialect now introduces a new compatibility surface; combining immutable bundle replacement would require a separate public identity design. Both remain later changes. Reusing generic validation tests alone leaves source failures implicit; add focused tests instead.

## Risks / Validation
Exact issue paths and rules are deliberate public validation output. Assert them without snapshotting message wording. No output snapshots change. Run model tests, strict OpenSpec validation, fast before checkpoint and ci before final review. A failure indicating missing production behavior requires scope review before edits. Test optional fields by comparing serialized cubes with their input so a defaulted/dropped field cannot pass a mere cross-format equality check.

## Delivery
Sync only the tested Cube fields requirement into existing contract-model main specs, preserving all existing requirements. Keep immutable model bundles in the seed. Archive in this PR before final review; obtain omp/Opus review and remote t0, then owner approves and merges. No runtime migration; revert tests/spec adoption to undo.
