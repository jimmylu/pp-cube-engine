# Design

## Decision

Call validate_model at the start of compile, before checking security filters or
query state. Return Model(Vec<ValidationIssue>) for any nonempty report. Reuse the
validator's sorted report unchanged, including programmatically mutated models.
The Display text is a fixed public message; structured issues remain available
for callers. This does not certify arbitrary UDF semantics or execute SQL.

## Alternatives

Caller-only validation leaves the current bypass. A validated-bundle wrapper
could avoid repeated work, but changes ownership and public input APIs. Direct
validation is the smallest complete boundary; caching can follow measured need.

## Compatibility

The compile signature and valid SQL/parameters are unchanged. The additive error
variant affects exhaustive matching. Invalid bundles now return model issues even
when the query/security filter is also invalid. Internal detail remains as before.
No parsing dependencies or lockfile changes are needed.

## Validation

Test only public compile and validate_model boundaries (already used by existing
regressions). Compare the full model report, exercise structural and SQL faults,
prove precedence over malformed security and ordinary query checks, preserve
input values and verify fixed Display text. Run existing compile goldens, fast,
CI and strict OpenSpec validation. Archive before final review in this PR.
