# service-api Specification

## Purpose
Expose compile-only operations through a typed, versioned model service boundary.

## Requirements

### Requirement: Compile validation and metadata operations
The Rust service core SHALL expose synchronous Compile, ValidateModel and GetMeta. Compile SHALL select an exact available version and return the facade result or a classified failure without execution. ValidateModel SHALL return structured issues without registration. GetMeta SHALL expose public cubes and members with names, types, titles and descriptions only. Absent public flags SHALL mean public. Unknown versions SHALL fail without SQL.
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
