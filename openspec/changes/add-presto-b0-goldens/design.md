# Design

## Context

See proposal.md for motivation. Main at planning time has the Presto templates and feature-specific golden suites. PR #34 adds their shared full-result replay wrapper, but is not merged yet. Read-only inspection of that reviewed head supplies the wrapper context; this branch starts at main.

The matrix is explicit in the seed's B0 boundary requirement: six aggregates, filtered measures, number formulas, joined dimensions, twenty filter operators, and/or groups, segments, absolute ranges, five granularities, timezone conversion, order, limit and offset. This change adopts only Presto's golden requirement; the boundary requirement and its unsupported-query policy stay in the seed.

## Goals / Non-Goals

Use existing executable cases as the Presto matrix, fill the non-UTC granularity gap and make its coverage reviewable. Do not create another runner or claim engine execution. No production change, cache, service, template rewrite or new dependency is needed.

## Decisions

### Reuse feature suites

The following existing tests supply the matrix; names below are in `crates/pp-cube-compile/tests/`. Mark these golden tests directly with B0-PRESTO-001, retaining their current scenario tags. The combined cases cover the matrix collectively; no single tag proves the whole matrix.

| Matrix members | Suite and test |
| --- | --- |
| count, count_distinct, sum, avg, min, max | aggregation: six_aggregate_types_grouped_by_a_dimension |
| Filtered measures, number formulas | aggregation: filtered_sum_and_ratio_of_aggregates |
| Joined dimension, joined filter | joins: sales_grouped_and_filtered_by_a_store_dimension |
| equals, notEquals, set, notSet | filters: equality_and_presence_operators |
| contains, notContains, startsWith, notStartsWith, endsWith, notEndsWith | filters: string_operators_escape_the_pattern_characters_of_their_values |
| gt, gte, lt, lte | filters: comparison_operators_on_a_number_dimension |
| Measure filtering / HAVING | filters: filters_on_measures_restrict_groups |
| and/or nesting | filters: top_level_filters_are_joined_with_and_and_groups_keep_their_nesting |
| Segment, explicit order, limit, offset | filters: segment_order_limit_and_offset |
| Six date operators | time: each_date_operator_compares_with_the_bounds_of_its_dates |
| Absolute closed range | time: october_is_closed_at_its_first_and_its_last_millisecond |
| Five granularities in UTC | time: each_granularity_truncates_the_time_dimension_in_the_timezone |
| Context timezone and query precedence | time: query_timezone_wins_over_the_one_of_the_context |
| Five granularities outside UTC | time: new per-granularity golden cases |

Duplicating the matrix in a standalone integration binary would duplicate fixtures and lose automatic coverage when feature tests evolve. Building T1/T2 now would require separate infrastructure and harness scope. Keep the existing tests and add only the missing inputs.

### Add independent non-UTC inputs

For each of day, week, month, quarter and year, compile sales.amount grouped by sales.sold_at with context timezone Asia/Tokyo and absolute range 2026-10-01 through 2026-10-31. Use the existing `compile_in`/`compiled` helpers in time.rs and five explicitly named snapshots. The new tests name B0-PRESTO-001 and B0-DET-001.

Snapshot SQL and ordered parameters as the suites already do. PR #34's wrapper compares every Compiled field over four fresh child processes for each successful input. Preserve the existing UTC snapshot and timezone-precedence cases. Review all five new snapshots with cargo insta review; none of the pre-existing snapshots is expected to change.

No production error behavior changes. Load/compile failures, nonzero worker exits, malformed worker output and differing records remain test failures through the existing helpers. If new inputs expose a production defect or pinned upstream contradiction, record it and stop under the scope rules rather than changing templates or accepting failed output.

### Transfer only the implemented tier

Move the complete Presto B0 compilation requirement and its id verbatim out of the seed. Keep Presto B0 acceptance / B0-PRESTO-002 and B0-PRESTO-003 there until their execution tiers exist and pass. Archive this change only after the entire table has passing reviewed goldens.

## Risks / Trade-offs

- Goldens freeze output, not engine validity → explicitly report T1/T2 as not run; their requirements remain proposed.
- Scenario tags can hide gaps → inspect the complete table and each operator/granularity input before archive.
- PR #34 is unmerged → planning can publish now; synchronize owner-merged main before implementation, preserving one implementation owner at a time.
- Snapshot size → review scope around 400 changed lines; keep these independently testable matrix cases together.

## Validation and Migration

Run strict OpenSpec validation for both active changes. Implementation runs the five golden suites with INSTA_UPDATE=no, repeats them with RUST_TEST_THREADS=1, then runs fast before checkpoint and ci before final review. Do not repeat the known unrelated result.rs single-thread failure (#36) as a prerequisite.

There is no runtime migration. Before final delivery, sync/archive only the tested Presto requirement, retain this coverage table in the archived design, and obtain omp/Opus review and exact-head t0. The owner approves and merges. Rollback removes the added tests/spec adoption; runtime behavior remains unchanged.
