## ADDED Requirements
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
