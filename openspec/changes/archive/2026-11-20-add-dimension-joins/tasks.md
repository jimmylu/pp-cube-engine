# Tasks

Each task is bounded to two hours.

## 1. Tracer: one join, end to end
- [x] 1.1 Give the planner each cube's join definitions and a join tree for two cubes joined directly; add the join templates; add the fixture and the golden test for `// scenario: B0-JOIN-001`; verify by reviewing the snapshot that there is one `LEFT JOIN` on the declared condition in a flat `SELECT`. Record in notes.md the templates asked for and how the planner asks for joins.

## 2. One root, one path
- [x] 2.1 Find the root and the path to each cube, through cubes the query does not name; add the golden test with a dimension two joins away; verify by reviewing the snapshot that the joins are in order.
- [x] 2.2 Refuse more than one root or path; add the test for `// scenario: B0-JOIN-002`; verify that the message names the cubes and that no path is chosen.
- [x] 2.3 Refuse cubes without a join path and a measure of a joined cube; add the tests for `// scenario: B0-JOIN-004`; verify the messages, for `many_to_one` and `one_to_one`.

## 3. Data sources
- [x] 3.1 Refuse a tree whose cubes are on more than one data source, cubes passed through included; add the test for `// scenario: B0-JOIN-003`; verify that the message names the cubes and no SQL is returned.

## 4. Close
- [x] 4.1 Verify that `Cargo.lock` is unchanged against `main` and that no existing snapshot changed.
- [x] 4.2 Archive; verify the traceability step reports the four scenarios as specified and named by tests.
