# Session notes
Owner authorized product bundle/version management after #44. Scope is model/catalog API and compile integration; no harness changes. Selected change add-versioned-model-bundles. No owner decision is open. Caller digest remains opaque metadata; same-version overwrite is prohibited. Implementation and focused/fast validation complete; SHA-bound delivery evidence belongs in PR comments.

Focused model API tests (2) and alternating compilation integration (1) passed.
The initial public API test failed on missing exported bundle types, before their
implementation. Tests prove located validator reports are preserved, registration
is non-destructive, exact lookup rejects unknown versions, handles outlive catalog,
and Arc copy-on-write cannot mutate bundle-owned models. No snapshot updates.
Fast passed (4 steps, 0 skipped); traceability passed. Spec archive/sync is part of this PR. Final delivery will repeat CI and local Docker before Opus review. Owner merges;
final head-bound evidence belongs in PR comments, not a tracked evidence-only commit.
