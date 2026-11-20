# Design

## Context and selected scope

PR #27 merged as main 8e85004. Read facade, adapter, join graph, query properties,
member symbols, top-level planner, public result design and seed deltas. The
adapter currently discards result metadata; TopLevelPlanner::plan returns only
SQL and pre-aggregation usage. Public QueryProperties/member symbols expose
members and aliases, while the logical planner chooses projection order.

Implement the whole successful-result requirement together with fingerprint
identity: the former already promises a fingerprint. Keep classified errors and
the all-golden independent-process suite in the seed, without claiming them done.

## Public result and context

Keep sql and parameters. Add columns: Vec<OutputColumn>, used_cubes: Vec<UsedCube>
and fingerprint: [u8; 32]. OutputColumn carries the exact unquoted output alias,
fully-qualified member and a public semantic type (string/number/boolean/time).
Time bucket columns retain the base member and identify their granularity so two
buckets cannot be confused. Measures have number semantics under the B0 model;
physical database precision is not inferred. UsedCube contains the model cube
name and its physical table sources, retaining qualified identifier components.
SQL text is never substituted for a physical table name.

Add request_id: String to CompileContext, following the public design. It is
correlation-only and does not enter planning or the fingerprint. Update existing
struct-literal tests explicitly. Keep the compile function signature unchanged.

## Projection authority and adapter boundary

Extend Planned with metadata. Before implementing, trace the pinned planner's
logical/physical projection and public schema accessors. Reuse resolved symbols
and their actual aliases in final projection order. Hidden filter/order members,
unbucketed time filters, duplicated selections, cross-cube formulas and count
fallbacks need explicit regression coverage. A missing metadata correspondence
is an internal error rather than a guessed column list.

Do not fork/reimplement TopLevelPlanner or infer projection order solely from
query input lists. If the pinned API cannot expose the final projection faithfully
without vendor changes, record the evidence and stop for an owner decision. This
is a required implementation feasibility check, not evidence of implementation.

Alternatives: parsing SELECT aliases from SQL alone loses member/type identity;
query-only metadata misses planner transformations. Prefer resolved planner
symbols paired with the actual result projection at the adapter seam.

## Source lineage

Report the cube sources actually consumed by the selected plan, including join
intermediates and implicit count dependencies. Exclude unused model cubes. Track
source accesses during planning through the adapter or use exposed final-plan
sources; do not report every graph node considered during validation/root search.
For sql_table, parse its qualified identifier without executing it. For cube sql,
extract the single physical source from the already-validated row SELECT.
Reuse the pinned parser through a narrowly scoped model helper if necessary.
Preserve quoted identifier components; reject ambiguous extraction as an internal
metadata error. A CTE/derived alias is not a physical table. Deduplicate cubes and
tables in a documented canonical order; columns retain projection order.

## Fingerprint encoding

Use SHA-256 with an explicit versioned domain separator and length-delimited,
canonical encoding. Never use DefaultHasher, debug formatting or ambiguous string
concatenation. Canonicalize object keys recursively; preserve ordered arrays,
including order entries and filter groups. Preserve numeric metadata precision.
Do not hash generated SQL as a substitute for hashing the required inputs.

The current compile API receives a typed Model, not a versioned raw-file bundle.
Its bundle digest is SHA-256 of the canonical complete typed model encoding,
including SQL and metadata; raw YAML/JSON formatting is intentionally absent from
that API. Record this definition in public API docs. If implementing it requires
changing the accepted model identity beyond this typed contract, stop for decision.

Hash a versioned record containing that model digest, the original query, dialect,
context timezone, security filters, kernel package version and full upstream pin.
The request id is excluded. Include semantic context separately from the cloned
query with appended security filters, avoiding accidental double hashing. Cover
the context timezone even if the query overrides it, as the requirement names
that input. Include no clock, random seed, credentials, external state or runtime IO.

Kernel/pin identity is a compile-time constant available to tests; source it from
an existing in-scope pin mechanism if present. Do not read Git state at runtime or
change harness configuration. Record how it stays aligned on upstream upgrades.
A new crypto dependency requires licence/advisory review and risk:upstream when
Cargo.lock changes. Hash serialization failures remain internal errors.

## Verification and compatibility

B0-RESULT-001: grouping by a region with distinct parameter values proves parameter
order and actual aliases, member types, physical tables and used cubes. Also cover
joined/intermediate cubes, cube sql, sql_alias, time buckets, repeated selections,
security-only dependencies and empty/non-selected time dimensions. Assert existing
SQL snapshots and parameter values unchanged; do not silently update snapshots.

B0-DET-002: demonstrate repeated identity and request-id exclusion, then vary each
covered input independently. Test kernel/upstream identity through a narrow pure
encoder seam with explicit identity inputs; production identity is fixed. At least
one integration case compares fingerprint equality across fresh processes. This is
focused identity evidence, not completion of B0-DET-001's all-golden suite.

Run model/adapter/compile tests, strict OpenSpec validation, fast and CI. Metadata
is successful-result information and may expose physical names as the public
contract requires; existing public error redaction rules remain in force.

## Risks and review

Projection/source observability is the first feasibility gate. No unsupported
planner internals are assumed to be public. Hash identity is versioned rather than
promising permanent binary compatibility. Extra public fields affect struct literal
callers. Scope may exceed 400 lines; keep one independently verifiable successful
result behavior together, and review size after the initial projection spike.
