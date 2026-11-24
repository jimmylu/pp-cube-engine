# Spec Delta

## ADDED Requirements

### Requirement: Queries on views
A query MAY name the members of a view wherever it names members: measures, dimensions, time dimensions, filters, security filters and order. The kernel SHALL compile them as the members the view takes, joined along the cubes of the model as any query is, as the pinned planner writes them: the result columns SHALL carry the view's names, and the used cubes SHALL be the cubes behind the view. View members and cube members MAY stand in one query.
#### Scenario: B1-VIEW-001 Members under the view's name
- **WHEN** a view's sum is grouped by its own and a prefixed joined dimension, and its measures are grouped by a dimension two joins away with a time dimension, a dimension filter, a measure filter, an order and a page
- **THEN** each SQL is the SQL of the same query on the cubes but for the column names, with equal parameters and used cubes, the columns name the view's members, and a view dimension compiles beside a cube measure

### Requirement: Views carry every capability of their members
A member taken by a view SHALL keep what it does in its cube: aggregation over repeated rows, rolling windows and time shifts along the view's time dimension, declared granularities, formulas, filtered measures and ungrouped rows.
#### Scenario: B1-VIEW-002 Capabilities through a view
- **WHEN** a view is queried for a sum over rows its join repeats, a rolling measure, a shifted measure with its base, a declared granularity, a formula with a filtered sum and a one_to_one measure, and ungrouped rows
- **THEN** each compiles to the SQL its members compile to, under the view's column names

### Requirement: View boundaries
A member the view does not take SHALL be a query error naming it. Every refusal that holds for the members behind a view SHALL hold through it, and a security filter on a view member SHALL restrict the cube behind it.
#### Scenario: B1-VIEW-003 Rules of the members behind
- **WHEN** queries name a member the view excludes or does not include, filter a view on a count over repeated rows, take a view's rolling measure without a time dimension, and carry a security filter on a view dimension two joins away
- **THEN** the first are query errors naming the member, the next two return unsupported naming the member behind the view, and the security filter is a condition on the joined cube
