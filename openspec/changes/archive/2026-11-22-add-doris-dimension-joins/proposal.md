## Why
Doris basic compilation currently rejects every model containing joins. Enable B0 dimension joins using the existing pinned planner and audited join boundary.
## What Changes
Allow multi-cube models and dimension joins on Doris. Preserve filters, ordered parameters, output metadata and physical lineage. SQL output changes only for newly enabled Doris requests; Presto remains unchanged.
## Capabilities
### Modified Capabilities
- `dialect-doris`: dimension joins and revised initial envelope.
## Impact
Scope: crates/pp-cube-compile/src/lib.rs, crates/pp-cube-compile/tests/doris.rs and its snapshots, this change and openspec/specs/dialect-doris/spec.md. No harness/vendor edits. No time/pattern support, joined measures, full B0 or Doris engine acceptance. Seed B0-DORIS-001/002 remain proposed.
