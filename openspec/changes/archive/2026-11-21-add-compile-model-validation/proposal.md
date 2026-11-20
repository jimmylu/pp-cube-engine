# Proposal

## Why

The public validators now detect baseline SQL/model faults, but compile does not
call them. A caller can bypass validation and pass an invalid typed bundle to the
planner. Close this boundary before designing raw-query unsupported errors or
complete result metadata.

## What Changes

- Validate the complete typed model before security/query checks and planning.
- Add CompileError::Model containing the existing ordered ValidationIssue report.
- Keep the public Display message free of model SQL and physical schema details.
- Preserve compiled output for valid models and expose structured model issues.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- model-validation: enforce validation at the public compile boundary.

## Impact

Scope: crates/pp-cube-compile/src/lib.rs, compile tests, model-validation spec and
this change folder. The additional public error variant requires exhaustive-match
callers to handle model errors. No successful SQL or snapshot output changes.

## Non-goals

Raw query parsing, unsupported/internal error redesign, complete compile-result
metadata/fingerprint, adapter changes, cached model bundles, vendor or harness.
The seed's classified-error and order-of-checks requirements remain proposed:
this change does not claim their full unsupported-feature scenarios.
