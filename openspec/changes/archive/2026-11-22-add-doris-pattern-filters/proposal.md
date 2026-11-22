## Why
Doris rejects B0 string search operations. Enable case-insensitive literal contains/prefix/suffix matching for user and security filters.
## What Changes
Add Doris LIKE templates and permit the six pattern operators through the existing recursive envelope check. Preserve literal wildcard escaping, bindings and NULL behavior. No Presto output changes.
## Capabilities
### Modified Capabilities
- `dialect-doris`: pattern filter compilation and revised boundary.
## Impact
Scope: templates/doris.yaml, crates/pp-cube-compile/src/lib.rs, crates/pp-cube-compile/tests/doris.rs and its snapshots, this change and openspec/specs/dialect-doris/spec.md. No harness/vendor edits, time support or Doris execution acceptance. Full B0-DORIS-001/002 remain in the seed.
