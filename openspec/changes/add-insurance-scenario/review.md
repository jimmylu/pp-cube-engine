# Go-live review and route

Assessment of main `e94bc19` on 2026-11-24, replacing the route in
`archive/2026-11-23-complete-compile-contract/review.md`, whose eight units are
done. It measures the kernel against going live, with the deployment facts the
owner gave the same day.

## Deployment facts from the owner

| Fact | Value |
| --- | --- |
| Engines in production | Presto 0.296, Doris 2.1.8, Druid 26.0 |
| Deployment | 3 replicas, to scale horizontally; the semantic layer publishes models |
| Model source | Replicas read published model files from shared storage: a NAS directory, and in enterprise environments object storage over the S3 protocol. Both are supported at once |
| Image registry | Not decided. The destination first named was meant for model files and was withdrawn |
| Callers | Mainly Java; Python and JavaScript also need support |
| Real models and queries | None yet; a realistic property-insurance scenario is to be simulated |
| New requirement | Model validation packaged as WebAssembly for the product frontend |

## Held

| Area | State |
| --- | --- |
| Compilation | B0, and of B1: multi-fact queries, rolling windows, period to date, time shifts, ungrouped rows, approximate distinct counts, declared granularities, views |
| Evidence | 70 shared cases executed on Presto 0.293 and Doris 4.1.3 and compared with Cube Core 1.8.28 on every engine-relevant pull request; about 170 one-off probe queries |
| Service | Six gRPC operations, read and admin tokens, TLS, health, JSON logs, Prometheus metrics, bounded queue, deadline, graceful drain, worker panic isolation |
| Performance | Prepared compilation p50 0.3 to 1.9 ms at 4 to 200 cubes; 1900 to 12000 per second on 4 threads (one laptop) |
| Packaging | Image built and verified in CI, aarch64 tested; version 0.1.0 in the manifests |

## Gaps against going live

| # | Area | State on main | Gap | Blocks go-live |
| --- | --- | --- | --- | --- |
| 1 | Engine versions | Accepted on Presto 0.293 and Doris 4.1.3 | Production runs Presto 0.296 and Doris 2.1.8. Doris 2.1 is two major versions behind what was tested: the floor functions with an origin, `APPROX_COUNT_DISTINCT`, `DATETIMEV2` casts, common table expressions and `DATE_TRUNC` argument order have not been run there. Nothing is known to fail; nothing is known to pass | Yes |
| 2 | Druid | Paused on 2026-11-23; no dialect, no templates, no cases | Druid 26.0 is in production and in scope (owner, 2026-11-24). Known from the design: multi-fact and time-shift SQL contains `UNION ALL` inside a subquery, which Druid refuses, so those queries would be unsupported there. Druid's join support is limited. Not started | Yes |
| 3 | Model source for replicas | Published versions live in one process's memory; a restart keeps only `--model` files | With 3 replicas a version published to one is unknown to the others, and a restarted replica has lost what was published. The service cannot read a directory of versions or an S3 bucket, does not notice a new version appearing there, and readiness does not say whether versions are loaded | Yes |
| 4 | Release | No git tag, no release, image never pushed, no SBOM or signature, no deployment manifest, no upgrade or rollback notes | Cannot be installed from a registry | Yes |
| 5 | Clients | `engine.proto` and one Rust example | No Java, Python or JavaScript client, no published stubs, no guide per language. JavaScript in a browser cannot speak gRPC without a proxy | Yes for Java |
| 6 | Fit to the domain | Features chosen from Cube's list, tested on a retail model | No model or query of the target domain has been tried. Which refusals matter is a guess | Yes: it orders everything below |
| 7 | Executed SQL performance | Not measured at all | Two shapes are suspect by inspection, not by measurement: the multi-fact join on `a = b OR (a IS NULL AND b IS NULL)`, which engines may not hash-join, and the enumerated time series on Doris (365 lines for a year by day) | Measure before go-live |
| 8 | Breadth of agreement | 70 hand-written cases and probes: agreement where checked | No randomized comparison with Cube Core, no fuzzing of the loaders, feature combinations thinly covered | Strongly advised |
| 9 | Deviations from Cube | About a dozen refusals and one model rule, each stated in its own pull request | No single list a caller can read | Advised |
| 10 | Service capacity | Compile function benchmarked | The gRPC service has no load or soak test and no memory figure per model version; a running compilation cannot be cancelled | Advised |
| 11 | Access | One read token, one admin token, rotated by restart | Callers are indistinguishable; no audit identity | Depends on the owner's network |
| 12 | Still refused | Relative date ranges, `compareDateRange`, sub-day granularities, weeks and quarters in Presto windows, segments in views, a view flag in GetMeta, filtered counts over repeated rows (#83), multi-stage ranks and shares, `one_to_many`, `case` dimensions | Unknown which the domain needs: see 6 | By the survey |
| 14 | Presto statement size | Found by the scenario (this change) | Presto carries a prepared statement in an HTTP header, also through JDBC; the default limit is 8 KB and a growth query by a dimension is 8.4 KB encoded. Such requests fail before running unless the server's header limit is raised or values are written into the statement. The execution tier has the same limit | Yes: a deployment setting and a client rule |
| 13 | Debt | Test helpers repeated across files; issues #31, #32, #36 | The batched test rework of rule 9 has not been done | No |

## The WebAssembly validator: is the direction sound?

Yes, with conditions.

Established on `e94bc19`: `pp-cube-model`, which holds loading and all model
validation, builds for `wasm32-unknown-unknown` unchanged. It reads no file,
clock, thread or environment, and its dependencies are pure Rust. The planner
is not involved in validating a model. So one set of rules can serve the
service and the browser with no second implementation.

Shape that keeps it sound:

- **A thin crate, not a fork.** A new `pp-cube-wasm` exposes
  `validate(text, format)` and the kernel version through `wasm-bindgen`, and
  is published as an npm package built from the same commit as the service
  image. No rule lives in it.
- **The service stays the authority.** The browser's answer is advice for the
  author; `ValidateModel` and `PublishModel` validate again. A model is never
  trusted because a frontend said so.
- **Version skew is visible.** The package and the service both state the
  kernel version; the frontend compares them and falls back to the service's
  answer when they differ.

What has to be built for it to be useful, which the server never needed:

- **Positions.** An issue carries a path into the document
  (`/cubes/0/measures/1/sql`), not a line and column, and a syntax error is
  reported at the root without a position. An editor needs both. Mapping
  paths to source spans belongs in `pp-cube-model`, where the service gains it
  too.
- **Stable rule ids are the contract.** Messages are English; the frontend
  translates by `rule_id`.
- **Scope stated plainly.** A valid model is not a guarantee that every query
  compiles: ambiguous join paths, dialect limits and the data-source mapping
  are decided per query or per deployment. The validator says "this model is
  inside the contract", nothing more.

Not recommended now: the planner or compilation in the browser. Its
dependencies were not examined for WebAssembly, the module would be far
larger, and compiled SQL in a frontend has no use the service does not serve.

Costs: a WebAssembly target and `wasm-bindgen` in CI are harness changes and
travel alone; the module's size and load time are unmeasured and should run in
a worker, loaded on demand.

Alternative considered: calling `ValidateModel` with a debounce. Always
consistent and nothing to ship, but it needs a round trip per keystroke and a
gRPC gateway for the browser. The two combine well: WebAssembly while typing,
the service on save.

## Route

Confirmed in order by the owner on 2026-11-24, with the decisions below. Each
row is one change of about 2 to 3 working days unless noted.

**Step 0, first: fit to the domain.**

| # | Change | Why first |
| --- | --- | --- |
| G0 | Property-insurance scenario (this change): a realistic model and a set of business queries, compiled for both dialects, with a support matrix of what compiles and what is refused, and execution cases | Turns gap 12 from a guess into a list and gives every later change a realistic fixture |

**Phase 1: blocks going live.**

| # | Change | Kind |
| --- | --- | --- |
| G1 | Production engine versions: run the cases on Presto 0.296 and Doris 2.1.8; fix the Doris dialect where 2.1 differs | Harness pins, then product |
| G2 | Model source: the service loads versions from a directory (NAS) and from an S3-protocol bucket, either or both, picks up versions the semantic layer adds or retires, verifies each digest, and reports loaded versions in readiness. The kernel still does no IO; the service does | Product |
| G3 | Druid 26.0, B0: dialect, templates, its own execution tier and cases, Cube Core comparison; 4 to 5 days | Product and harness |
| G4 | Druid 26.0, B1: what of B1 Druid can answer, and by name what it cannot (multi-fact and time shifts need `UNION ALL` in a subquery) | Product |
| G5 | Release: tag, image, SBOM and signature, deployment manifest for 3 replicas, upgrade and rollback notes. Needs a registry (open question) | Harness |
| G6 | Java client: generated stubs as an artifact, a thin typed wrapper, a guide; then Python and Node.js; browsers through a gateway | Product |
| G7 | Executed SQL: plans and timings of multi-fact and rolling SQL on production-sized data, on each engine, against Cube Core's SQL for the same query. Where a different text is measurably faster, the kernel may deviate (decision 5) | Investigation, then product |

**Phase 2: assurance, in parallel with phase 1.**

| # | Change | Kind |
| --- | --- | --- |
| G8 | Randomized comparison with Cube Core over the scenario model, and fuzzing of the loaders | Harness and product |
| G9 | One list of deviations from Cube and of refusals, for callers; decide #83 | Docs |
| G10 | Service load and soak test, memory per version, capacity guide | Harness |
| G11 | Batched test rework (rule 9) | Product tests |

**Phase 3: the validator and what the survey asks for.**

| # | Change | Kind |
| --- | --- | --- |
| G12 | Source positions for validation issues and syntax errors | Product |
| G13 | `pp-cube-wasm` and its npm package | Product, with a harness change for the build |
| G14+ | Features the scenario shows are needed, in its order | Product |

## Decisions

Answered by the owner on 2026-11-24:

1. **Druid 26.0 is part of going live.** This ends the pause of 2026-11-23 for
   Druid; other dialects stay out.
2. **The image destination first named is withdrawn.** It was meant for
   published model files, which now come from shared storage instead.
3. **Replicas read models from shared storage:** a NAS directory, and object
   storage over the S3 protocol in enterprise environments; both supported.
4. **The WebAssembly validator is phase 3.**
5. **Deviation from Cube Core's SQL text is allowed where measured
   performance warrants it, provided the result is better than Cube Core's.**
   So a deviation needs a measurement on the production engine against Cube's
   SQL for the same query, equal results, and an entry in the deviation list;
   the Cube Core comparison then accepts that case by result, not by text.

Still open:

- Which container registry the service image is published to (G5).
