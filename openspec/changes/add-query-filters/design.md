# Design

## Context
`compile` takes a query with `measures` and `dimensions` only; any other field fails to load. The adapter hands the planner a query options object whose filters, order, limit and offset are all empty, and has no segment definition.

Read from the pinned planner, not run:
- Filters arrive as a tree of `{member, operator, values}` leaves and `{and}`/`{or}` groups in the options' static data. The planner knows every operator of the baseline, and also `in`, `notIn` and `onTheDate`, which the baseline does not have.
- Filter SQL is rendered from the dialect's `filters/*` templates; values become `params/param` placeholders, cast through `tesseract/*_param_cast` for number and boolean members. Presto overrides the pattern template used by contains and its relatives.
- The planner reads the row limit from `rowLimit`, not `limit`, and takes limit and offset as strings.
- A segment is a definition with one expression, wrapped by `expressions/wrap_segment_filter`.

## Goals / Non-Goals
**Goals:** the four requirements of the delta on Presto: value filters, their grouping, segments with order and row limits, and the refusal of invalid filters.

**Non-Goals:** anything about time; joins; filters supplied by the compile context; classification of failures beyond query and internal.

## Decisions
- **Filter values are strings.** Cube's query format defines `values` as an array of strings, numbers included. A number or boolean in `values` fails to load. Alternative rejected: accepting JSON numbers, which adds a second spelling of the same query and the float questions already settled for `meta`.
- **A filter is an enum: a condition or a group.** `{member, operator, values}`, `{and: [...]}` or `{or: [...]}`; an object mixing them fails to load. The operator is a closed enum of the fourteen value operators, so `in`, `notIn` and any unknown word are refused at load with the position, and the six date operators are refused until time dimensions are adopted.
- **Order is kept as written.** Cube accepts an object or an array of pairs. A JSON object's key order survives only if it is read as a sequence, so `order` has its own deserializer that accepts both forms and yields pairs in document order. Alternative rejected: the array form only; the object form is the one Cube's documentation leads with.
- **Type fit is checked in the adapter, before planning,** next to the existing member check: string operators on string dimensions, comparison operators on number dimensions and measures, the four general operators on anything. The planner does not check this; it would emit SQL that fails on the engine. The rule follows Cube's documentation of which operators each member type takes.
- **`set` and `notSet` take no values; every other operator needs at least one.** Checked with the type fit.
- **A filter on another cube's member is refused by the existing join check,** as a query over two cubes is today.
- **Templates are added as the planner asks for them,** as in the previous change: a missing one fails with an error naming it, and the test that hits it says which to transcribe.
- **Golden tests:** one snapshot per operator family rather than one per operator, each compiling several operators in one query, so the matrix of `B0-QUERY-006` is fully exercised without fourteen files. `B0-QUERY-005` and `B0-QUERY-007` get one snapshot each. The parameters are part of each snapshot.
- **Validation:** unit tests for loading the query type, the golden tests naming the four scenario ids, tests for each refusal, `scripts/check.sh ci`.

## Risks / Trade-offs
- The facts above come from reading → the first task compiles one `equals` filter end to end before anything else is built on them.
- Presto does not reuse parameters, so a value used twice is bound twice → the snapshots show the parameter list; the order is asserted, not assumed.
- The seed's requirement text is re-cut in the same pull request → the table in proposal.md lists every id and where it goes; strict validation and the id audit run on the result.
- Like patterns are escaped by the dialect's template → the snapshot for the string operators includes a value with `%` and `_`.
- `B0-QUERY-006` is a matrix scenario → its tests compile all fourteen operators.

## Migration Plan
The query type gains fields; a query that loads today loads unchanged. On archive, `openspec/specs/contract-query/spec.md` is created with the four requirements.
