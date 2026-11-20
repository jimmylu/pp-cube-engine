# Proposal

## Why

The loaders stop at the first decoding problem, while reference and join faults can survive until compilation. Callers need a complete, located list of structural model issues before requesting SQL.

## What Changes

- Add document validation for YAML/JSON and structural validation for typed models, reporting object path, stable rule id and message.
- Check required fields, field types and allowlists, member references, join targets, primary keys on both ends and one join per cube pair.
- Transfer only the complete "Structured complete validation" requirement, retaining B0-VALID-001, from the seed. SQL/expression restrictions remain proposed in the seed.
- Keep existing strict loaders and compile interfaces compatible; this slice introduces explicit validation, without claiming all baseline validation is complete.

## Capabilities

### New Capabilities

- `model-validation`: accumulated structural model diagnostics.

### Modified Capabilities

None. Existing contract-model and query/join compilation requirements remain in force.

## Impact

Planning paths: `openspec/changes/add-model-validation/**` and `openspec/changes/seed-baseline-specs/specs/model-validation/spec.md`.

Implementation paths: `crates/pp-cube-model/src/{lib,load,validation}.rs`, narrowly scoped reuse of `reference.rs`, and model validation tests/fixtures. On completion, archive this folder and create the corresponding main spec in the same Draft PR under repository workflow.

B0 only. Valid model loading and generated SQL remain unchanged; the new validation API returns multiple diagnostics instead of stopping at one. No dependency change is expected.

## Out of Scope

SQL AST/expression validation (B0-VALID-002 through B0-VALID-004), new dialects, result metadata/fingerprints, error-class overhaul, service endpoints, automatic compile rejection through a validated-model type, B1, issues #22/#23, SQL execution, vendor and harness edits. This session creates planning artifacts only.
