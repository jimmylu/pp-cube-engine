# Tasks

Each task is bounded to two hours.

## 1. Tracer: one granularity, end to end
- [ ] 1.1 Add `timeDimensions` with a dimension and a granularity, and `timezone`, to the query type; add `timezone` to the compile context; verify that they load from Cube query JSON and that `hour` and an unknown granularity fail to load with a position.
- [ ] 1.2 Pass time dimensions and the effective timezone to the planner; resolve the predefined granularities; add the `driver/` templates and render them from the adapter; compile `month` in UTC; verify by reviewing the snapshot. Record in notes.md the templates asked for, the default order, and the member form an order on a time dimension takes.

## 2. Granularities and timezone
- [ ] 2.1 Add the golden test with the five granularities for `// scenario: B0-QUERY-001`; add the test for `// scenario: B0-QUERY-008`; verify by reviewing the snapshot.
- [ ] 2.2 Check the timezone names of the query and the context before planning; add the golden test for `// scenario: B0-QUERY-010`; verify that the two compilations differ in the zone they name and that an unknown name is a query failure.

## 3. Date ranges and date operators
- [ ] 3.1 Add the date value type and `dateRange`; implement the bound conversion after `localTimestampToUtc`; verify it by a unit test against bounds worked out by hand, one inside a clock change; add the golden test for `// scenario: B0-QUERY-003`.
- [ ] 3.2 Add the six date operators to filters; add their golden test for `// scenario: B0-QUERY-001`; verify by reviewing the snapshot, including the parameter order.

## 4. Refusals
- [ ] 4.1 Refuse a relative or single-string `dateRange` and a malformed date value when the query is read; add the test for `// scenario: B0-QUERY-004`; verify that each fails with a position and that no clock is read.
- [ ] 4.2 Check before planning: a time dimension that is not of type `time` or asks for nothing, a date operator on another member, and the number of values each date operator takes; verify that each is a query failure naming the member or the rule.

## 5. Close
- [ ] 5.1 Compare `Cargo.lock` with `main`; verify that it gains edges for `pp-cube-adapter` only and that `cargo deny` passes.
- [ ] 5.2 Archive; verify the traceability step reports the five scenarios as specified and named by tests.
