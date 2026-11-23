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
The binary SHALL require a loopback listen address, a dialect for every data source of its models, and one or more models each given as an exact version and a path, unless model management is enabled. It SHALL validate every model before readiness, expose the three existing gRPC operations and announce the bound address. Invalid startup SHALL exit unsuccessfully without readiness. Ctrl-C SHALL stop the server gracefully.
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
