# contract-model Specification

## Purpose
Define the Cube-compatible model inputs accepted by the compile kernel.

## Requirements

### Requirement: Dimension and measure fields
The kernel SHALL accept exactly the fields below, plus the optional metadata on both and format on measures. It SHALL preserve `meta` without interpreting it.
- dimension: required name, sql, type (string, number, boolean, time); primary_key; granularities
- measure: required name, type (count, count_distinct, count_distinct_approx, sum, avg, min, max, number); sql, required unless count; filters; rolling_window; multi_stage; time_shift
#### Scenario: B0-MODEL-001 Equivalent model formats
- **WHEN** equivalent YAML and JSON models define every dimension type, measure type and optional field above
- **THEN** both load with equivalent definitions and unchanged metadata

### Requirement: Joins, segments and references
A join SHALL have exactly name, relationship and sql, and a segment exactly name and sql, all required. Expressions SHALL refer to columns and members as `{CUBE}.column`, `{member_of_this_cube}` or `{other_cube.member}`.
#### Scenario: B0-MODEL-004 Join, segment and reference forms
- **WHEN** a model declares a join, a segment and expressions using each reference form
- **THEN** each loads and resolves to its declared cube, column or member

### Requirement: Exact metadata numbers
A number in `meta` SHALL load only when it can be kept exactly: an integer within the 64-bit range, or a finite float below 2^63 in magnitude. Any other number SHALL fail to load, from YAML and from JSON alike.
#### Scenario: B0-MODEL-005 Inexact metadata number
- **WHEN** `meta` holds a non-finite number, a float of magnitude 2^63 or more, or an integer outside the 64-bit range
- **THEN** loading fails in either format, while the widest 64-bit integers load unchanged

### Requirement: Cube fields
The kernel SHALL accept exactly these cube fields, in YAML or equivalent JSON: name, sql_table, sql, data_source, sql_alias, joins, dimensions, measures, segments, and the optional metadata title, description, public and meta. A cube MUST have a name, a data_source and exactly one of sql_table or sql.
#### Scenario: B0-MODEL-002 Source selection
- **WHEN** a cube omits its name or data_source, or supplies both or neither of sql_table and sql
- **THEN** validation reports a model issue at the offending cube path
#### Scenario: B0-MODEL-006 Cube field boundary
- **WHEN** equivalent YAML and JSON documents supply every optional cube field with either sql_table or sql, and another document adds an unknown cube field
- **THEN** both valid forms load equivalently with their fields preserved and validate without issues, while the unknown field produces a model issue at its field path

### Requirement: Immutable model bundles
A model bundle SHALL own a validated set of models, an exact nonblank version and a nonblank caller-supplied digest. The kernel SHALL expose only shared read access to bundle contents. Digests SHALL be retained as opaque caller metadata without claiming content verification; compilation fingerprints SHALL continue to reflect semantic model content. A new version replaces the complete model set selected by the caller, never part of an existing bundle.
#### Scenario: B0-MODEL-003 Bundle replacement
- **WHEN** two bundle versions that define the same cube differently are compiled alternately
- **THEN** each result reflects its own bundle's definition and neither bundle changes

### Requirement: Validated catalog registration
The catalog SHALL accept only validated bundles, retain multiple versions and reject duplicate version registration without replacing existing contents, even when digests agree. Blank version or digest identifiers SHALL fail; nonblank identifiers SHALL be retained without normalization. Invalid model bundles SHALL report the existing structured validation issues and SHALL NOT become available or affect available versions.
#### Scenario: B0-MODEL-007 Invalid registration isolation
- **WHEN** a valid version is registered, an invalid model and blank identifiers are offered, and the valid version is selected again
- **THEN** invalid input is rejected with classified errors, model issues retain locations, and the valid version remains unchanged and selectable

### Requirement: Explicit version selection
Catalog lookup SHALL return a shared immutable bundle for the exact requested version or an unknown-version error without selecting another version. Duplicate registration SHALL return a duplicate-version error. Existing bundle handles SHALL remain usable after additional registrations and after the catalog is dropped.
#### Scenario: B0-MODEL-008 Exact selection and handle lifetime
- **WHEN** a duplicate version is offered with different contents, an unknown version is selected, another version is registered, and the catalog is dropped
- **THEN** duplicate and unknown selection fail, the original version is not replaced, and retained handles preserve their versions, digests and model contents

### Requirement: Catalog listing and retirement
The catalog SHALL list its bundles ordered by version and SHALL retire a version on request. Handles obtained before a retirement SHALL keep their bundle unchanged. A retired version SHALL be unknown to later selection and MAY be registered again. Retiring an unknown version SHALL fail.
#### Scenario: B0-MODEL-009 List and retire
- **WHEN** three versions are registered out of order, one is retired while a handle to it is held, and it is registered again
- **THEN** listing is ordered by version, the held handle keeps its contents, the retired version cannot be selected or retired again, and the new registration is available

### Requirement: Measure windows and shifts
A measure MAY carry `rolling_window`, with `trailing`, `leading` and `offset`, or with `type: to_date` and `granularity`; `multi_stage`; and `time_shift`, a list of entries with `interval`, `type` and an optional `time_dimension`. They SHALL load as written from YAML and JSON. A measure that has none of them SHALL serialize without them, so the content digest of a model that uses none is unchanged.
#### Scenario: B1-MODEL-001 Windows and shifts load
- **WHEN** a model declares a bounded window, a to_date window, a shifted multi-stage measure and a multi-stage formula over it, in YAML and again as JSON
- **THEN** each loads with its fields as written, both formats give the same model, and a measure without these fields serializes without them

### Requirement: Declared granularities and approximate counts
A time dimension MAY declare `granularities`, each with a name, an interval and optionally an origin or an offset, and a measure MAY have the type `count_distinct_approx`. They SHALL load as written from YAML and JSON. A dimension without granularities SHALL serialize without the field, so the content digest of a model that declares none is unchanged.
#### Scenario: B1-MODEL-002 Granularities and approximate count load
- **WHEN** a model declares a granularity with an origin, one with an offset and an approximate distinct count, in YAML and again as JSON
- **THEN** each loads with its fields as written, both formats give the same model, and a dimension without granularities serializes without the field

### Requirement: Views
A model MAY declare `views`. A view SHALL have a name and `cubes`, each with a `join_path` written `root.joined.joined`, `includes` as member names or `*`, and optionally `excludes` and `prefix`; it MAY have title, description, public and meta. A model without views SHALL serialize without the field. Expanded, a view SHALL be a cube whose members refer by join path to the dimensions and measures it takes, named `<cube>_<member>` under `prefix`.
#### Scenario: B1-MODEL-003 Views load and expand
- **WHEN** a model declares a view that takes named members of its root cube, all but one member of a joined cube under a prefix, and one member two joins away, in YAML and again as JSON
- **THEN** both formats give the same model, the expansion adds one cube whose dimensions and measures refer to the members taken by their join paths, with the declared granularities of a time dimension and no primary key, and a model without views serializes without the field
