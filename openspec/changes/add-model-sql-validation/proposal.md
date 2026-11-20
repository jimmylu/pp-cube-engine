# Proposal

## Why

Structural validation now reports definition faults, but a structurally valid model can still contain joins, aggregation or other forbidden constructs in cube SQL. Complete the remaining B0 model-validation requirement before expanding results or dialects.

## What Changes

- Extend the existing validation APIs with SQL AST and expression checks, reporting located issues without rewriting model text.
- Adopt the complete "Baseline SQL and joins" requirement and B0-VALID-002 through B0-VALID-004 from the seed; preserve their ids.
- Reject forbidden cube SQL, aggregate calls outside measures and cross-cube physical-column access; retain structured rejection of unsupported model fields/relationships.
- Use the parser approach already specified in public kernel design section 6.1.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `model-validation`: add the remaining baseline SQL/expression restrictions.

## Impact

Planning: this change folder and the seed model-validation delta, deleted after its last complete requirement transfers here.

Implementation: `crates/pp-cube-model/src/{validation,sql_validation}.rs`, narrowly scoped reference helpers if needed, model tests/fixtures, crate Cargo.toml and Cargo.lock for the new parser. No workspace/toolchain/harness or vendor edits. Apply the repository's `risk:upstream` label if Cargo.lock changes, without changing the upstream pin.

Existing valid B0 models retain byte-identical source text and generated SQL. The validation APIs will reject previously unchecked SQL constructs. Strict loaders and compile signatures remain compatible; automatic facade validation is separate.

## Out of Scope

SQL execution, SQL rewriting, physical-schema/catalog inspection, authorization, new dialect compilation, complete result metadata, fingerprinting, service APIs, new compile error classes, B1, deferred joins #22/#23, vendor and harness changes. This session creates planning documents only.
