# B0 progress and route

Assessment of main `7c20185` on 2026-11-22. Presto B0 compilation, validation,
security, metadata, fingerprints, pin identity and T0 determinism have evidence.
Cube Core comparison, real execution, Doris/Druid dialects, service APIs and T1/T2
remain open. A Tokyo aggregation can produce stable Presto SQL, but semantic
equality and execution are unproven; `ungrouped: true` and `hour` are now
classified as Unsupported while unknown input stays strict. Model-version
isolation and service selection remain open; multi-fact/time-shift/rolling
windows remain B1.

Route: complete this compiler boundary; build Presto Cube Core/T1/T2 evidence;
add Doris and Druid acceptance; then immutable bundles and Compile/ValidateModel/
GetMeta before declaring B0 and proposing B1. Issues #22, #23, #30, #31, #32 and
#36 remain deferred.
