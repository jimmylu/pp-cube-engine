# Design

## Context

`Query` previously rejected recognized Cube options such as `ungrouped` and
sub-day granularities during loading. The facade validates the model before
query checks, and the adapter is also callable directly. Vendor code remains
read-only and B0 does not execute SQL.

## Goals / Non-Goals

Expose a finite typed boundary, preserve grouped behavior and model-first errors.
Do not implement detail output, comparisons, sub-day grouping, B1 planning,
execution tiers, service APIs, dialects or a raw JSON compile entry point.

## Decisions

- Add optional `ungrouped` and `compareDateRange` carriers and Second/Minute/Hour
  enum variants. Absent fields are omitted from serialization; false remains a
  grouped request. Unknown names and malformed values remain load errors.
- Use one `Query::unsupported_b0_features` predicate in facade and adapter.
  Model validation runs first. Recognized boundary options return Unsupported;
  adapter direct calls fail before planner options are built.
- Multi-cube Unsupported requires a valid unique join tree on one data source;
  join/member/security validation errors take precedence. Existing goldens and
  fingerprints must not change.

## Risks / Validation

The public query structs and enum are source-breaking for exhaustive clients;
all workspace literals and matches are updated. Tests cover every recognized
boundary option, unknown/malformed controls, model precedence, adapter direct
calls, grouped-false SQL/parameters and both multi-cube error classes.

## Migration

The implementation, spec sync and archive are in this PR. Reverting restores
load-time rejection. T1/T2 remain unbuilt; final delivery needs exact-head CI,
cross-vendor review and remote t0 before owner approval.
