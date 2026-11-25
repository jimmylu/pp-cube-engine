# service-api Specification

## Purpose
Expose compile-only operations through a typed, versioned model service boundary.

## Requirements

### Requirement: Compile validation and metadata operations
The Rust service core SHALL expose synchronous Compile, ValidateModel and GetMeta. Compile SHALL select an exact available version and return the facade result or a classified failure without execution. ValidateModel SHALL return structured issues without registration. GetMeta SHALL expose public cubes and members only, never SQL or physical tables. Absent public flags SHALL mean public. Unknown versions SHALL fail without SQL.
#### Scenario: B0-API-001 Service operations
- **WHEN** an available version is compiled, an unavailable version is named, an invalid bundle is validated, and metadata containing public and non-public members is requested
- **THEN** compilation returns the facade result, the unavailable version fails without SQL, validation returns located issues, and metadata excludes non-public cubes and members

### Requirement: Validate before availability
The service SHALL validate a bundle version before making it available to Compile and GetMeta, and SHALL NOT make an invalid version available. Versions already available SHALL be unaffected.
#### Scenario: B0-API-002 Invalid bundle version
- **WHEN** the service is given an invalid bundle version while a valid version is available
- **THEN** the validation issues are reported, the invalid version cannot be selected, and the valid version still compiles

### Requirement: Compile request propagation
The service SHALL pass the selected bundle, query, dialect, timezone, security filters and request id to the facade without dropping or weakening filters.
#### Scenario: B0-API-003 Security filters across the service
- **WHEN** Compile receives a user OR group with a context security filter, and then receives a malformed security filter
- **THEN** the first result requires the security filter AND the entire user OR group, and the second returns a classified failure without SQL

### Requirement: Dedicated bounded compilation
The service SHALL offer fixed dedicated compilation workers with bounded waiting capacity. Submission SHALL return immediately with a task or a Busy error, without running the planner on the caller thread. Awaiting a task SHALL preserve the complete versioned service result or return Unavailable if execution unwinds. Zero workers or queue capacity SHALL be rejected.
#### Scenario: B0-API-005 Asynchronous compilation parity
- **WHEN** a current-thread runtime submits different versions, a security-restricted query and an unknown version
- **THEN** results and classified failures equal direct service calls without blocking the runtime thread
#### Scenario: B0-API-006 Bounded dispatch and recovery
- **WHEN** workers are occupied, the queue fills, a task panics and a result handle is dropped
- **THEN** excess admission returns Busy, work runs only within the fixed worker budget, and workers remain reusable

### Requirement: Worker lifecycle
Dropping the pool SHALL close admission without waiting for compilation. Accepted work SHALL drain before workers exit. Explicit shutdown SHALL wait for accepted work and join workers; it SHALL be documented as blocking and requiring execution outside async runtime threads.
#### Scenario: B0-API-007 Graceful worker shutdown
- **WHEN** a pool with accepted work is dropped or explicitly shut down
- **THEN** accepted results remain receivable, explicit shutdown joins the workers, and drop does not wait for compilation

### Requirement: Network service transport
The service SHALL expose Compile, ValidateModel and GetMeta through gRPC, with compilation dispatched to a fixed-size worker pool outside async runtime threads.
#### Scenario: B0-API-004 Network operations
- **WHEN** a gRPC client sends Compile, ValidateModel and GetMeta requests
- **THEN** the server maps them to the versioned service operations and dispatches compilation off the async runtime threads

### Requirement: Local service process
The binary SHALL require a listen address, a dialect for every data source of its models, and one or more models each given as an exact version and a path, unless model management is enabled. It SHALL validate every model before readiness, expose the service operations and the health protocol, and announce the bound address. Invalid startup SHALL exit unsuccessfully without readiness. An interrupt or terminate signal SHALL stop the server gracefully.
#### Scenario: B0-API-008 Runnable local service
- **WHEN** the binary starts with a valid model and explicit identity on a loopback socket
- **THEN** the announced address serves Compile, ValidateModel and GetMeta for that version, and Ctrl-C exits successfully
#### Scenario: B0-API-009 Invalid process startup
- **WHEN** arguments are malformed, a model is unavailable or invalid, a data source of the model has no dialect, or a socket cannot bind
- **THEN** the process exits unsuccessfully without announcing readiness

### Requirement: Typed wire contract
The gRPC contract SHALL carry each parameter as text, number, boolean or null in placeholder order; each used cube with its data source and its tables as path components; the declared session properties; and a failure kind of model, query, unsupported or internal. Compile and GetMeta responses for an available version SHALL carry an identity: kernel version, upstream pin, model version and model digest. An unavailable version SHALL be a not-found status.
#### Scenario: B0-API-010 Wire contract
- **WHEN** a client compiles a filtered query, a paginated Presto query, an invalid query and an unimplemented dialect, and requests metadata
- **THEN** it receives typed ordered parameters, structured used cubes, the session property only for the paginated query, enumerated failure kinds, and the same identity on each response

### Requirement: Data source dialects
The service SHALL know which dialect serves each data source. It SHALL NOT make available a model that names a data source without one. Compile SHALL return SQL only when the requested dialect serves every data source the compiled query reads, including cubes a security filter brought in; otherwise it SHALL return a query failure naming the cube, the data source and the dialect, without SQL or table names.
#### Scenario: B0-API-011 Dialect and data source
- **WHEN** a model on two data sources served by different dialects is compiled for each dialect, and a model on an unmapped data source is offered
- **THEN** each cube compiles only for the dialect of its data source, the mismatches are query failures without SQL, and the unmapped model does not become available

### Requirement: Discoverable metadata
GetMeta SHALL return, for each public cube, its name, title, description, data source, segments, model meta as written, and joins to public cubes with their relationship; for each public measure its type, title, description, format and meta; and for each public dimension its type, title, description, primary-key flag and meta. Absent public flags SHALL mean public.
#### Scenario: B0-API-012 Metadata content
- **WHEN** metadata is requested for a model with hidden cubes and members, segments, formats, meta and a join to a hidden cube
- **THEN** it lists the public content with those fields, omits hidden cubes, hidden members and the join to the hidden cube, and contains no SQL expression or table name

### Requirement: Model version lifecycle
The service SHALL publish, list and retire model versions while serving. Publishing SHALL validate first and SHALL NOT affect other versions. A version SHALL be immutable: publishing the same content again SHALL succeed without change and report that nothing was created; different content under an available version SHALL be refused. Retiring SHALL make the version unknown to new requests without failing compilations that already selected it. Listing SHALL be ordered by version.
#### Scenario: B0-API-013 Publish, list and retire
- **WHEN** versions are published, republished with the same and with different content, listed, compiled while others are published and retired, and retired
- **THEN** equal content is accepted without creating anything, different content is refused, listing is ordered, concurrent compilations of an available version all succeed, and a retired version is not found while the others still compile

### Requirement: Content digest
The service SHALL identify a version by a digest of its typed content, written as sha256: and 64 lowercase hex digits, equal for the same content in YAML or JSON and different for different content. ValidateModel SHALL return the digest of a valid model. A publish that states an expected digest SHALL publish nothing unless it equals the content digest.
#### Scenario: B0-API-014 Digest of content
- **WHEN** one model is written as commented YAML and as reordered JSON, a changed model is digested, and versions are published with a wrong and with the right expected digest
- **THEN** the two forms have one digest, the changed model another, the wrong expectation publishes nothing, and the right one publishes a version whose identity carries that digest

### Requirement: Opt-in management
ListModels, PublishModel and RetireModel SHALL be refused with a permission error unless the host enabled model management, which is off by default. Compile, ValidateModel and GetMeta SHALL be unaffected by the setting.
#### Scenario: B0-API-015 Management disabled
- **WHEN** a client calls the three management operations on a service that did not enable them
- **THEN** each is refused with a permission error, nothing is published, and validation and metadata requests are still answered

### Requirement: Access tokens
The gRPC service SHALL support a read token and an admin token presented as bearer credentials. With a read token configured, Compile, ValidateModel and GetMeta SHALL require the read or the admin token. With any token configured, management operations SHALL require the admin token. A missing or unknown credential SHALL be an authentication error, a read token on a management operation a permission error. With no token configured, callers are trusted. The health protocol SHALL need no token.
#### Scenario: B0-API-016 Token roles
- **WHEN** clients call with no credential, another scheme, an unknown token, the read token and the admin token
- **THEN** the first three are unauthenticated, the read token compiles and reads metadata but is denied management, the admin token may do both, and a health probe is answered without a token

### Requirement: Health protocol
The service SHALL answer the standard gRPC health Check for the whole server and for its own service name with serving, and with not serving once the host has begun to drain. An unknown service name SHALL be not found. Watch SHALL be unimplemented.
#### Scenario: B0-API-017 Health check
- **WHEN** a probe checks the server, the service name and an unknown name, calls Watch, and checks again after draining has begun
- **THEN** it receives serving twice, not found, unimplemented, and then not serving

### Requirement: Request telemetry
The service SHALL count every finished request by operation and outcome, record its duration, and emit one structured log line with operation, outcome, duration and, when present, request id, model version, known dialect and result fingerprint. Telemetry SHALL NOT contain SQL, parameter or filter values, table names, model content or credentials. Caller-chosen text SHALL be length-limited and unable to split or forge a line.
#### Scenario: B0-API-018 Counted and logged
- **WHEN** a restricted query compiles, an invalid query fails, an unknown version is named and an unauthenticated call is refused
- **THEN** each produces one log line and one count under its outcome, the lines carry the listed fields, and neither lines nor metrics contain the filter value, table name, SQL, members or tokens

### Requirement: Bounded requests
The service SHALL bound how long a caller waits for a compilation, the size of a request, and the number of available versions. A compilation that overruns SHALL return a deadline error and the service SHALL keep working. An oversized request SHALL be refused before it is interpreted. Publishing beyond the version limit SHALL be refused until a version is retired; republishing available content SHALL not count.
#### Scenario: B0-API-019 Limits
- **WHEN** a compilation cannot start within the deadline, a request exceeds the size limit, and a version is published beyond the limit and again after one is retired
- **THEN** the first is a deadline error after which requests succeed, the second is refused, the third is refused and then accepted

### Requirement: Deployable host
The binary SHALL listen beyond loopback only with a TLS certificate and key and a read token, SHALL read tokens from files, and SHALL refuse management with tokens but no admin token. It SHALL optionally serve metrics in the Prometheus text format on a separate address, log request events as JSON lines on standard error, and on a terminate or interrupt signal report not serving, finish running requests within a drain time, and exit.
#### Scenario: B0-API-020 Encrypted host
- **WHEN** the binary is started beyond loopback without TLS or without a token, and then with both, a metrics address and management
- **THEN** the first starts fail without readiness, and the last serves encrypted identified callers, refuses plaintext and unidentified ones, answers a probe, serves metrics, logs JSON events, and exits successfully on a terminate signal

### Requirement: Views in the service
Compile SHALL accept queries on the views of a published version and return the facade result. GetMeta SHALL list each public view after the cubes, as a cube without joins or segments: its public members under their names in the view, each measure with the type of the measure it takes.
#### Scenario: B1-API-001 View compilation and metadata
- **WHEN** a version with a view is published, a query on the view is compiled, and metadata is requested
- **THEN** the compilation equals the facade's, and the view is listed last with its title, its data source, the public members it takes and the type of the measure behind each

### Requirement: Model store layout
A model store SHALL hold versions at `<business line>/models/<version>/`, where the business line has one to three levels. `models` is the asset type the service reads; keys under another name at that place SHALL be left alone. A version is complete when its `manifest.json` exists, and is named to callers by its business line and version joined by `/`. A `models` directory out of place and a malformed version name SHALL be reported, not guessed at.
#### Scenario: B1-STORE-001 Business lines and asset types
- **WHEN** a store holds complete versions under one, two and three levels of business line, a version without a manifest, another asset type, stray files, and `models` at the root, under four levels, under an upper-case line and with a malformed version
- **THEN** the three complete versions are found under their line and version, the incomplete one and the other asset type are left alone, and each misplaced or malformed one is reported as a layout fault

### Requirement: Versions from a store
The service SHALL offer every complete version of its store whose files are a model directory inside the contract, on data sources it has a dialect for, and whose content equals the manifest's digest when one is given. A version that cannot be offered SHALL be reported with one reason of a fixed set and SHALL NOT stop the others. Reports, logs and metrics SHALL name versions, files and rules, never model content.
#### Scenario: B1-STORE-002 Offered and rejected versions
- **WHEN** a directory store holds two valid versions, one with a matching digest, and versions with a wrong digest, an unknown manifest schema, a manifest that is not JSON, a stray file, a misnamed file, an invalid model and an unmapped data source
- **THEN** the valid ones compile by their business line and version, each faulty one is rejected with its own reason and counted by reason, and hidden entries and keys outside the store's root are not read

### Requirement: Following a store
Each later pass over the store SHALL offer versions that became complete and retire versions it offered whose manifest is gone. A loaded version SHALL NOT be read again. Versions made available another way SHALL NOT be retired by the store, and a pass that cannot list the store SHALL change nothing and be counted. A version that stays rejected for the same reason SHALL be counted in every pass and logged once.
#### Scenario: B1-STORE-003 Added, retired and unreachable
- **WHEN** a version's files appear and then its manifest, a loaded version's files change, a manifest is removed, a store version is retired through the service, the store becomes unreadable, and a rejected version is passed over again with the same and then with another reason
- **THEN** the version is offered only with its manifest, the changed one is unaffected, the one without a manifest is retired while a version from elsewhere stays, the retired one returns at the next pass, the failed pass leaves every version in place, and the rejection is logged again only when its reason changes

### Requirement: Store in the process
The process SHALL take a directory as its model store and a refresh interval. It SHALL make the first pass before announcing its address and fail to start when the store cannot be listed; a version that cannot be offered SHALL NOT prevent starting.
#### Scenario: B1-API-002 A replica on a store
- **WHEN** the process starts on a store with one valid and one broken version, and while it runs a new version is published and the first one's manifest removed; and separately it is started on a missing directory, on a store of an unbuilt kind, and with a refresh interval but no store
- **THEN** the valid version compiles as soon as the address is announced, the new one compiles within the refresh interval, the retired one stops compiling, and the three faulty invocations exit without announcing an address

### Requirement: S3-protocol store
A model store MAY be an S3-protocol bucket under a prefix, with the layout of a directory store. The service SHALL list it with ListObjectsV2 over as many pages as it takes and read objects with signed requests, taking keys relative to the prefix and leaving out folder markers and hidden entries. A failure SHALL name what was asked and the HTTP status, never the server's text.
#### Scenario: B1-STORE-004 Bucket under a prefix
- **WHEN** a bucket holds two complete versions under a prefix with a folder marker, a hidden entry and keys of another prefix, its listing takes several pages, an object is missing, and the bucket then refuses requests
- **THEN** the two versions are offered and compile as from a directory, every request is signed for that bucket, the missing object and the refusal are reported by status alone, and the refused pass changes nothing

### Requirement: Store credentials
The service SHALL hold no storage secret and take none as an option. It SHALL sign with access keys from the standard environment variables, or without them with the instance role's credentials from the metadata service, asked with a session token and kept for a while. The region, a custom endpoint and the metadata endpoint SHALL come from their standard variables. Half a key pair and web identity credentials SHALL be refused by name.
#### Scenario: B1-STORE-005 Environment and instance role
- **WHEN** a store is configured from variables with keys, a session token, a region and two endpoints, from none, from half a key pair and from a web identity token file, and a store without keys is listed and read several times
- **THEN** the first uses the keys, region and the S3 endpoint, the second is AWS with the default region and the instance role, the others are refused, and the keyless store asks the metadata service once and signs with the role's key and session token

### Requirement: Several stores
The process SHALL take any number of model stores, each a directory or `s3://bucket/prefix`, read every one before announcing its address, and number them in its metrics. A bucket that cannot be listed at startup SHALL stop the process, and a store of another scheme SHALL be refused.
#### Scenario: B1-API-003 A directory and a bucket at once
- **WHEN** the process starts on a directory and a bucket, and separately on a bucket that does not exist and on a store of another scheme
- **THEN** the versions of both stores are loaded before the address is announced and each store's pass is counted under its number, no secret is logged, and the two faulty invocations exit without announcing an address

### Requirement: Shipped deployment
The repository SHALL hold a deployment of three replicas whose service arguments start a serving replica as written. It SHALL replace one replica at a time and keep at least two available, probe a port that answers only once models are validated, allow longer to stop than the service drains, run as a non-root user with a read-only filesystem, and name the image of the current version, never a mutable tag.
#### Scenario: B1-API-004 The manifest's arguments
- **WHEN** the service is started with the arguments of the shipped manifest, its mounted certificate, tokens and model store made available, and is then sent SIGTERM
- **THEN** it announces that it listens, the probed path answers 200 and reports the store's model version, and it exits successfully; and the manifest states three replicas, one unavailable, at least two available, a grace period longer than the drain, and the current version's image
