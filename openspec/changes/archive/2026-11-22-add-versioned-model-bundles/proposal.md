## Why
The compile facade accepts one model but has no validated, immutable version selection boundary. Callers cannot register versions without inventing their own replacement and isolation rules.

## What Changes
Add ModelBundle and ModelCatalog to pp-cube-model. Bundles own validated model sets, exact version strings and caller-supplied digest strings. Registration never overwrites a version; lookup fails explicitly for unknown versions. Existing compile accepts the selected model unchanged. Transfer B0-MODEL-003 from the seed proposal and add registration scenarios.

## Capabilities
### Modified Capabilities
- `contract-model`: immutable versioned bundles and validated catalog selection.

## Impact
B0 only. Paths: crates/pp-cube-model, crates/pp-cube-compile/tests and openspec. No SQL/snapshot or fingerprint changes, no harness paths, dependencies or vendor edits. Non-goals: network service, persistence, implicit latest selection, concurrent writes, digest verification and new dialects.
