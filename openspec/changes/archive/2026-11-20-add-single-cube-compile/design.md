# Design

## Context
Read from the pinned planner (`vendor/cube/rust/cube/cubesqlplanner/cubesqlplanner/src`, "P" below), not run:

- **Entry.** The public path is `State::try_new` → `QueryPropertiesCompiler::build` → `TopLevelPlanner::plan` → `State::build_sql_and_params`, as the fixtures use it (`P/test_fixtures/test_utils/test_context.rs:620-634`). It returns `(String, Vec<FilterValue>)`. The other entry, `BaseQuery`, needs a Node context.
- **Interfaces.** A single-cube query reaches twelve bridge traits: query options, evaluator, base tools, driver tools, template renderer, member SQL, cube, measure and dimension definitions, join graph, join definition, and the empty security context. Pre-aggregation lookups are called on every query and must return nothing. About thirty other traits are reached only by features outside this change.
- **Fixtures.** Upstream's own implementations are test-only (`#[cfg(test)]`, `P/lib.rs:6-7`), so they cannot be a dependency. The files this path needs are about 6,500 lines, of which 1,560 are a mock schema builder and 1,000 are default templates in PostgreSQL style.
- **Templates.** The planner asks a renderer for templates by name and renders them with `minijinja`. No dialect templates exist in Rust; Presto's are in Cube's JavaScript, which is not vendored: `BaseQuery.ts` for the base set and `PrestodbQuery.ts` for overrides. Presto keeps `"` quoting and `?` parameters, does not reuse parameters, and overrides `statements/select`.
- **Ownership.** Everything the planner holds is `Rc<dyn Trait>` and its state is per query. Errors are `CubeError`, user or internal.

## Goals / Non-Goals
**Goals:** SQL for `B0-AGG-001` and `B0-AGG-002` on Presto, through the unmodified planner, from contract types. Seams that later changes extend without rewriting: one place for dialect data, one for bridge objects, one entry function.

**Non-Goals:** the rest of the query vocabulary, other dialects, result metadata, caching, thread model. A complete Presto template set.

## Decisions
- **Write the bridge objects against the contract types; do not copy the fixtures wholesale.** The kernel design proposes copying about 10,000 lines and then replacing their types. For this path that means importing a builder and a YAML layer only to delete them. Upstream code is copied only where the logic is not obvious, such as turning an expression into the planner's `{arg:N}` template, and then with its notice, path and commit. Alternative rejected: the wholesale copy; it would also bring thirty trait implementations nothing here can test.
- **Tracer first.** The first task drives one `sum` grouped by one dimension end to end with the thinnest possible objects. Linking, the macro-generated trait methods and the planner's real calls are checked before anything is widened. If it cannot be made to work within its task, that is a stop condition, not a reason to patch `vendor/`.
- **A dialect is data plus settings, behind one type.** `pp-cube-dialect` owns the template text; the adapter's driver tools and renderer delegate to it. Each dialect is one data file, `templates/presto.yaml`: the upstream commit at the top, then each template with its upstream file. Provenance cannot live inside a template, because the planner reads some of them raw, such as the quote character. The adapter never contains SQL text.
- **Transcribe only the templates this path renders.** A request for any other template, or for a helper function not yet written, fails with an error naming it, which the facade reports as an internal failure. Alternative rejected for this change: extracting Presto's full set now; most of it could not be tested here, and the extraction procedure belongs with the change that completes a dialect.
- **Golden tests with `insta`, held at 1.43.** One snapshot per scenario, reviewed before commit. Assertions on SQL substrings would restate the planner; a snapshot records what it emits and makes any later difference visible. Releases after 1.43 depend on `tempfile`, whose `rustix` switches on a feature of `bitflags`, one of the planner's dependencies; 1.43 adds two packages and changes none.
- **The dialect comes from the compile context.** A cube's `data_source` is not read here. How the two relate is one of the questions the seed leaves open and this change does not need it.
- **`compile` takes a shared model, not a bundle,** and returns SQL and parameters only. The planner keeps its objects for the length of a query and cannot borrow, so the model is an `Arc` and is never copied. Bundles, columns, used cubes and the fingerprint belong to requirements that stay in the seed.
- **Errors.** The members a query names are checked against the model before planning, so the message names `cube.member`; the planner's own message leaves out the cube. A `CubeError` marked user becomes a query failure; anything else becomes an internal failure that keeps its detail for the operator and does not show it in its message. The four-way classification is `B0-RESULT-002`, not adopted here, so a query over two cubes is reported as an internal failure for now.
- **`pp-cube-model` reports where each reference is.** The planner wants an expression with `{arg:N}` in place of each reference, so the adapter needs the bytes each one occupies; `locate_references` gives them and `parse_references` is built on it.
- **No change to how the planner's dependencies are built.** `minijinja` is added with exactly the planner's version requirement and features; `insta` is a dev-dependency of our crates. Task 4.1 compares every package's resolved features before and after.
- **Validation:** unit tests per crate, the two golden tests naming their scenario ids, and `scripts/check.sh ci`.

## Risks / Trade-offs
- The facts above came from reading, not running → the tracer confirmed them: the bridge traits can be implemented outside the planner, the test binary links against system libraries only, and the first query returned SQL without a change to the design. Its findings are in notes.md.
- Default templates are PostgreSQL style and Presto's `statements/select` differs → the golden SQL is Presto text nobody has executed; the snapshots say so, and execution stays with tiers T1 and T2.
- Hand-transcribed templates can drift from upstream → each file names its source and commit; the upgrade procedure compares them.
- About 1,900 lines in one pull request → it is one Draft with separate commits for the tracer with the two scenarios, and for the rest.
- `B0-AGG-001` is a matrix scenario → its test compiles all six aggregate types.

## Migration Plan
None; nothing uses these crates yet. On archive, `openspec/specs/compile-b0/spec.md` is created with this requirement.
