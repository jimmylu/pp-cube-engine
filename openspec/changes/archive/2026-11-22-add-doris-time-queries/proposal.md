# Doris time queries

## Why

Doris temporal requests are currently rejected, leaving the B0 time matrix incomplete.

## What Changes

Complete the B0 temporal compilation path for Doris: time dimensions, five B0
calendar granularities, named timezones and date filters. SQL output gains
Doris DATETIMEV2 casts, CONVERT_TZ and DATE_TRUNC. Parameters remain ordered.
Scope: templates/doris.yaml, compile facade boundary and its tests, Doris specs.
No harness/vendor changes, new baseline, execution in the kernel or full T2 claim.

B0-DORIS-010 formal tier acceptance remains in the active seed-baseline-specs
Doris delta. Its opt-in execution test is already implemented and passed locally;
definition and automatic invocation of its tier require separate harness work.
This PR edits that seed delta to keep the execution requirement proposed.
