# Design

## Why not the console first

1. It would test itself. A console is a new system: a back end, a front end, a query builder, a model editor. Its defects would mix with the kernel's, and the question "is the kernel right in this environment" would get a less clear answer.
2. Weeks against days, before knowing whether the real environment holds.
3. The kernel executes nothing, so a console needs a back end that holds the read token and the warehouse credentials and decides the row-level rules. That is a new service to review.
4. Editing models online goes around the design: versions are immutable and published by the semantic layer; an editor is what the WebAssembly validator of phase three is for.
5. A copy of another product's interface is not something to build. A later console would be of its own design.

## Decisions

1. **Two tracks.** Track A loads the property-insurance benchmark into a schema of the test's own and runs questions with answers worked out by hand: a definite pass or fail. Track B uses the tester's own model and queries, compared by a person with an existing report. The kit serves both: a case may have an expectation, a stated refusal, or neither.
2. **The kit does what a caller's back end does.** It compiles through the client, hands the parameters to the driver apart from the statement, applies the session properties and reads rows by the kernel's column names. Presto through `presto-python-client`; Doris through PyMySQL with the client's `sql_as('format')`; Druid through its SQL API with parameters and query context.
3. **One benchmark, any engine.** The rows are `insurance.json`; the kit writes each engine's DDL and loads them. On Druid each table is a SQL ingestion, and it waits as the Druid tier learned to: task succeeded, new segment served, no older one, columns known to SQL.
4. **The model is rendered for the engine.** `model` rewrites the example's model for the configured schema, table prefix and data source, and on Druid the timestamp column, and writes it in the store layout with the manifest last. It refuses to write over a version.
5. **Safe on a shared engine.** Only nine tables named `ins_*`, with the configured prefix, in the configured schema, are ever created or dropped. `load` changes nothing when any exists unless told `--replace`; `clean` needs `--yes`; `load --dry-run` prints the statements for someone with the right to run them. On Druid, which has one schema, a prefix is mandatory.
6. **No secret in the configuration or the report.** A password is named by an environment variable, a token by its file. The report holds queries, SQL, parameters and rows.
7. **Druid's NULL mode is read, not probed.** The first version asked a query (`'' IS NULL`); run against a Druid in legacy mode it answered "sql-compatible", because the constant is folded when the query is planned. `env` now reads `druid.generic.useDefaultValueForNull` from the engine's `/status/properties`, and says "unknown" when it may not.
8. **What an engine answers differently is stated.** `suites/insurance-dialects.json` lists it: Presto refuses year to date by quarter and the others answer it; Druid refuses ungrouped rows. A Rust test compiles every question for every dialect and requires the suite's expectation to be the kernel's behaviour, and the figures the guide prints (15/2, 16/1, 15/2) to follow.
9. **The guide cannot drift from the kit.** A Rust test reads every `ppcube-itest …` line of the guide and the README and requires the command and each option to exist, and every command to be shown.
10. **Chinese for the guide, English for the tool.** The owner asked for the guide in Chinese. The tool's output stays in the repository's language; the guide explains each line a tester will see.

## Verification

- Real containers, locally: track A on Presto 0.296, Doris 2.1.8 and Druid 26.0.0 through a locally run service; and on the example's stack over TLS with a token.
- The guide's chapters 3 to 7, 10 and 11 were walked through command by command and their stated expectations compared with what appeared. Chapters 1, 2 and 8 need a release and a cluster and are marked as not verified here.
- The kit's unit tests (14) and the two Rust tests.

## Not in this change

- CI for the kit's Python tests: a harness change.
- A console. If non-engineers need one after the test: model version picker, member tree, drag to measures, dimensions and filters, execution, and the request, SQL and parameters side by side; no model editing; a thin back end on the kit's code that keeps the credentials.
