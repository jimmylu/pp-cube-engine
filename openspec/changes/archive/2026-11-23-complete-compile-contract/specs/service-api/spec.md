# Spec Delta

## MODIFIED Requirements

### Requirement: Compile validation and metadata operations
The Rust service core SHALL expose synchronous Compile, ValidateModel and GetMeta. Compile SHALL select an exact available version and return the facade result or a classified failure without execution. ValidateModel SHALL return structured issues without registration. GetMeta SHALL expose public cubes and members only, never SQL or physical tables. Absent public flags SHALL mean public. Unknown versions SHALL fail without SQL.
#### Scenario: B0-API-001 Service operations
- **WHEN** an available version is compiled, an unavailable version is named, an invalid bundle is validated, and metadata containing public and non-public members is requested
- **THEN** compilation returns the facade result, the unavailable version fails without SQL, validation returns located issues, and metadata excludes non-public cubes and members

### Requirement: Local service process
The binary SHALL require a model path, exact version, opaque digest, loopback listen address and a dialect for every data source of the model. It SHALL validate the model before readiness, expose the three existing gRPC operations and announce the bound address. Invalid startup SHALL exit unsuccessfully without readiness. Ctrl-C SHALL stop the server gracefully.
#### Scenario: B0-API-008 Runnable local service
- **WHEN** the binary starts with a valid model and explicit identity on a loopback socket
- **THEN** the announced address serves Compile, ValidateModel and GetMeta for that version, and Ctrl-C exits successfully
#### Scenario: B0-API-009 Invalid process startup
- **WHEN** arguments are malformed, a model is unavailable or invalid, a data source of the model has no dialect, or a socket cannot bind
- **THEN** the process exits unsuccessfully without announcing readiness

## ADDED Requirements

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
