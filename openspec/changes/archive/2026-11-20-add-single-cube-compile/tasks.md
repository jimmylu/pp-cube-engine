# Tasks

Each task is bounded to two hours.

## 1. Tracer: one query, end to end
- [x] 1.1 Add the query type (`measures`, `dimensions`) to `pp-cube-model`; verify that it loads from Cube query JSON and rejects an unknown field.
- [x] 1.2 Add the dialect type to `pp-cube-dialect` with a template store rendered by `minijinja`; verify that a known template renders and an unknown one fails naming it.
- [x] 1.3 In `pp-cube-adapter`, implement the thinnest bridge objects for one cube with one `sum` measure and one dimension, and drive the planner's public entry in a test; verify that the test binary links and returns SQL. Record in notes.md which trait methods and template names the planner actually called. If this cannot be done without touching `vendor/`, stop.
- [x] 1.4 Add `compile` to `pp-cube-compile` over 1.1 to 1.3; verify that the same query returns SQL and an empty parameter list.

## 2. Single-cube aggregation
- [x] 2.1 Transcribe the Presto templates that 1.3 recorded into `templates/presto.yaml`, each entry naming its upstream file, with the commit at the top; verify that the tracer query renders with them.
- [x] 2.2 Complete the measure and dimension definitions for all six aggregate types; add the golden test for `// scenario: B0-AGG-001`; verify by reviewing the snapshot.
- [x] 2.3 Support measure filters and `number` formulas, including references between measures; add the golden test for `// scenario: B0-AGG-002`; verify by reviewing the snapshot.
- [x] 2.4 Support a cube defined by `sql`; verify that its SQL selects from the cube's query as a subquery.

## 3. Failures
- [x] 3.1 Map planner errors: a user error becomes a query failure naming the member; anything else becomes an internal failure that does not show its detail; verify with unknown members and with a query over two cubes, which the kernel does not compile yet.
- [x] 3.2 Keep panics out of the bridge objects; verify with `cargo clippy` and a search for `unwrap`, `expect` and `panic` outside tests.

## 4. Close
- [x] 4.1 Compare each package's resolved features from `cargo metadata` before and after this change; verify that no existing package changes and that `cargo deny` passes.
- [x] 4.2 Archive; verify the traceability step reports the two scenarios of `compile-b0` as specified and named by tests.
