# Tasks

Each task is bounded to two hours. Implementation starts after pull request #17 has merged.

## 1. Tracer: one query, end to end
- [ ] 1.1 Add the query type (`measures`, `dimensions`) to `pp-cube-model`; verify that it loads from Cube query JSON and rejects an unknown field.
- [ ] 1.2 Add the dialect type to `pp-cube-dialect` with a template store rendered by `minijinja`; verify that a known template renders and an unknown one fails naming it.
- [ ] 1.3 In `pp-cube-adapter`, implement the thinnest bridge objects for one cube with one `sum` measure and one dimension, and drive the planner's public entry in a test; verify that the test binary links and returns SQL. Record in notes.md which trait methods and template names the planner actually called. If this cannot be done without touching `vendor/`, stop.
- [ ] 1.4 Add `compile` to `pp-cube-compile` over 1.1 to 1.3; verify that the same query returns SQL and an empty parameter list.

## 2. Single-cube aggregation
- [ ] 2.1 Transcribe the Presto templates and helper functions that 1.3 recorded into `templates/presto/`, each file naming its upstream file and commit; verify that the tracer query renders with them.
- [ ] 2.2 Complete the measure and dimension definitions for all six aggregate types; add the golden test for `// scenario: B0-AGG-001`; verify by reviewing the snapshot with `cargo insta review`.
- [ ] 2.3 Support measure filters and `number` formulas, including references between measures; add the golden test for `// scenario: B0-AGG-002`; verify by reviewing the snapshot.
- [ ] 2.4 Support a cube defined by `sql`; verify that its SQL selects from the cube's query as a subquery.

## 3. Failures
- [ ] 3.1 Map planner errors: a user error becomes a query failure naming the member; anything else becomes an internal failure without the planner's message; verify with an unknown measure and with a template missing from the dialect.
- [ ] 3.2 Replace every panic path in the bridge objects with a returned error; verify with `cargo clippy` and a search for `unwrap`, `expect` and `panic` outside tests.

## 4. Close
- [ ] 4.1 Compare each package's resolved features from `cargo metadata` before and after this change; verify that no existing package changes and that `cargo deny` passes.
- [ ] 4.2 Archive; verify the traceability step reports the two scenarios of `compile-b0` as specified and named by tests.
