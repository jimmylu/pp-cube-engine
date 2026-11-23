# Productization review and route

Assessment of main `894fecd` on 2026-11-23, replacing the route in
`archive/2026-11-22-add-execution-case-manifest/review.md`. Owner direction of
the same day: pause new dialects, Druid included; make the existing capability
production-ready and feature-complete first.

## Held

B0 compiles for Presto and Doris; forty shared cases execute on both engines on
GitHub for every engine-relevant pull request. Validation, security filters,
fingerprints, determinism, three gRPC operations and a bounded worker pool exist.

## Gaps found in the code

| Area | State | Gap |
| --- | --- | --- |
| Wire contract | Untyped parameters, flattened tables, string failure kinds | Addressed by this change |
| Dialect and data source | Dialect per request; data source ignored | Addressed by this change |
| Metadata | Names, types, titles only | Addressed by this change |
| Model lifecycle | One file, one version at startup; digest unverified | No multiple versions, no runtime publish or retire |
| Deployment | Loopback only; no TLS or authentication | Cannot run as a network service |
| Operability | No health check, logs or metrics; no deadline, cancellation or input limits | Not observable, not self-protecting |
| Performance | Planner context rebuilt on every compile; no measurements | Latency and throughput unknown |
| Release | No image, version or release process; CI on Linux x86_64 only | Cannot be handed to a user |
| Semantic assurance | No Cube Core comparison (B0-PRESTO-002); #22, #23, #30 open | Goldens show stable output, not agreement with Cube |
| Features | Every B1 feature returns unsupported | Multi-fact, time shift, rolling window, views, ungrouped, approximate distinct, custom granularity |

## Route, confirmed by the owner

Each row is one change of about 2-3 working days (AGENTS.md rule 8).

1. Complete compile contract (this change).
2. Model bundle lifecycle: multiple versions, verified digest, runtime publish and retire.
3. Deployable and operable: non-loopback listen with TLS and an authentication hook, health, structured logs, metrics, deadlines, limits, graceful drain.
4. Performance and release: per-thread planner context cache, benchmarks and load test, container image and release process, aarch64 build, user documentation.
5. Semantic assurance: Presto against Cube Core (T1); #22, #23, #30.
6. B1 multi-fact queries.
7. B1 time shift, rolling window and period to date.
8. B1 views, ungrouped queries, approximate distinct, custom granularities.

Then dialect expansion. Every B1 change carries Presto and Doris goldens and
execution cases. Test or framework rework is batched separately (rule 9).
