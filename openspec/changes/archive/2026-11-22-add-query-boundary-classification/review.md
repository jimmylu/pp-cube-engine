# B0 progress and route to acceptance

Assessment of main `7c20185` on 2026-11-22 from the main specs, facade/model/
dialect code, service/conformance stubs and verification tiers. This is source
evidence, not an engine result. Seed task completion means proposal preparation.

## Present gap

The product goal is a compile-only kernel: Cube model, query and context become
target SQL plus ordered parameters. Presto B0 compilation, validation, security,
metadata, fingerprints, pin identity and deterministic replay have T0 evidence.
Cube Core comparison and real execution are still absent. `DialectId` only has
Presto; Doris and Druid templates and acceptance are absent. The service entry
point only prints its version, and T1/T2 commands remain unbuilt.

For example, a Tokyo month aggregation with a tenant filter can produce stable
Presto SQL and parameters, but equality with Cube Core or an engine result is
unproven. The same query cannot yet target Doris or Druid. `ungrouped: true` and
`hour` were previously indistinguishable from malformed `fortnight`; this change
classifies recognized options as Unsupported while preserving strict unknown
input. Model-version isolation and service `model_version` selection remain open.
Multi-fact, time-shift and rolling-window requests remain intentionally B1.

## Route

1. This PR closes typed B0 boundary classification and model-first errors.
2. Build a pinned Cube Core comparator and T1/T2 harness, first proving a
   Presto aggregate/filter/timezone slice, then the full B0 matrix.
3. Add Doris B0 and Druid single-cube dialects with separate execution evidence;
   Cube Core is not a Doris reference.
4. Specify immutable version/digest bundles, then implement service
   Compile/ValidateModel/GetMeta and full context propagation.
5. Declare B0 accepted only after dialect, execution, service and required T3
   evidence; propose B1 afterward.

After-baseline issues #22, #23, #30, #31, #32 and #36 remain deferred.
