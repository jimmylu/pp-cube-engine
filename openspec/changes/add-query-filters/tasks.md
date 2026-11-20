# Tasks

Each task is bounded to two hours.

## 1. Tracer: one filter, end to end
- [ ] 1.1 Add `filters` with a condition and the `equals` operator to the query type; verify that it loads from Cube query JSON and that a number in `values` fails to load.
- [ ] 1.2 Pass filters to the planner through the query options; compile `equals` on a string dimension; verify by reviewing the snapshot that the predicate is in WHERE and the value is the one parameter. Record in notes.md the templates the planner asked for.

## 2. Value filters and grouping
- [ ] 2.1 Add the remaining thirteen operators to the query type and the templates they render; add the golden tests for `// scenario: B0-QUERY-006`, one per operator family, on dimensions and on a measure; verify by reviewing the snapshots, including the parameter order.
- [ ] 2.2 Add `and`/`or` groups to the query type; add the golden test for `// scenario: B0-QUERY-005`; verify by reviewing the snapshot.

## 3. Segments, order and row limits
- [ ] 3.1 Add `segments` to the query type and a segment definition to the adapter; verify that the segment's condition is in WHERE.
- [ ] 3.2 Add `order`, read in document order from either form, with `limit` and `offset`; add the golden test for `// scenario: B0-QUERY-007`; verify by reviewing the snapshot.

## 4. Invalid filters
- [ ] 4.1 Check filters before planning: unknown member, operator not fitting the member's type, values missing or not allowed; add the test for `// scenario: B0-QUERY-002`; verify that each is a query failure naming the member or the rule.
- [ ] 4.2 Verify that an operator outside the baseline and a date operator each fail to load with a position.

## 5. Close
- [ ] 5.1 Compare each package's resolved features from `cargo metadata` with `main`; verify that no existing package changes and that `cargo deny` passes.
- [ ] 5.2 Archive; verify the traceability step reports the four scenarios of `contract-query` as specified and named by tests.
