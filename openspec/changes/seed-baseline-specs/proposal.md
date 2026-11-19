# Proposal

## Why
The public engine design needs testable B0 contracts before implementation. This proposal establishes the vocabulary and acceptance targets without claiming that the empty workspace implements them.

## What Changes
- Draft twelve capability deltas from sections 3–8 and appendix A of the public design (`docs/design/engine-design.md`, `docs/design/engine-kernel.md`).
- Reserve stable scenario ids for later implementation changes; ids marked `(T1)` or `(T2)` need the execution tiers of `docs/agents/verification.md`.
- Keep these deltas unarchived until corresponding behavior and tests exist, following owner decision #5.
- Multiple capabilities are necessary to seed the common contract; future implementation changes remain small and capability-focused.

## Capabilities
### New Capabilities
- `contract-model`: Cube model fields and immutable bundles.
- `contract-query`: B0 queries and absolute time ranges.
- `compile-result`: SQL, parameters, metadata and errors.
- `model-validation`: structured baseline validation.
- `compile-b0`: aggregation, formulas, dimension joins and the B0 feature matrix.
- `security-filters`: mandatory and caller-provided filters, failing closed.
- `dialect-presto`: Presto B0 output.
- `dialect-doris`: Doris B0 output.
- `dialect-druid`: Druid single-cube B0 output; joins unsupported.
- `service-api`: compile, validation and public metadata.
- `determinism`: repeatable compilation and fingerprints.
- `upstream-pin`: the upstream commit behind every compiled result.
### Modified Capabilities
None; no main specs exist yet.

## Impact
Only `openspec/changes/seed-baseline-specs/**` belongs in this proposal PR. Public design publication and `openspec/config.yaml` are prerequisites delivered separately, after pending harness work. No SQL output changes occur in this proposal.

## Out of Scope
Product implementation, archive, snapshots, vendor edits, harness edits, SQL execution, credentials, authorization decisions, migration, B1 and post-baseline extensions. Capability implementation requires separately approved changes; this proposal is not blanket authorization to implement B0.
