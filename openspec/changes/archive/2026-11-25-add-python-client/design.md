# Design

## Decisions

1. **The same shape as the Java client, in Python's idiom.** `Client.compile(...)` with keyword arguments returns a frozen dataclass; a query may be a `dict`; the client is a context manager. The four mistakes it removes are the same: splicing values into SQL, dropping session properties, retrying a refusal, sending a token in clear.
2. **Stubs are generated when the package is built**, by `generate.py` from `crates/pp-cube-service/proto`, into `src/ppcube/_contract/`, which is ignored by git. There is no copy of the contract and no generated file to forget to regenerate. The cost: the package builds only from a checkout, and the build needs `grpcio-tools`. `setup.py` exists only to run that step.
3. **`protoc`'s absolute import is rewritten.** The generated gRPC module imports its sibling as a top-level module, which fails inside a package. `generate.py` rewrites that one line and fails loudly if the line is not there, so a change in the generator's output cannot pass silently.
4. **Placeholders are converted by a scanner, not by `str.replace`.** DB-API drivers disagree on placeholder style: Presto's takes `?`, MySQL-protocol drivers for Doris take `%s`. `CompiledQuery.sql_as('format' | 'numeric')` walks the statement, skips quoted text and quoted identifiers (single, double, backtick, doubled quotes), and for `format` doubles every other `%`. If the count of placeholders it finds differs from the number of parameters, or a quote does not close, it raises: it never returns a statement it is unsure of. This is the Python counterpart of the Java client's `bind`.
5. **Named after the contract.** The module is `ppcube`, after `ppcube.engine.v1`; the distribution is named after the project with `-client`, like the Java artifact.
6. **A failed call stays `grpc.RpcError`; a refusal is `CompileRefused`.** `kind` is a lower-case string (`model`, `query`, `unsupported`, `internal`); an unknown kind from a newer service is `internal`.
7. **One example, two back ends.** The Python back end reads the same `portal.json`, the same environment variables and the same warehouse address, and ends with the same sentence. A reader can compare them line for line, and a check can run either.
8. **Python 3.9.** The oldest version still widely deployed that `grpcio` supports; it is also what this change was run on.

## Verification

- T0: Rust test B1-CLIENT-003, without Python.
- Local, in this change: the client's nine unit tests against a stand-in gRPC server, and the example end to end with the Python back end on real containers.
- Not in CI yet: a harness change will add both.

## Not in this change

- Publishing to an index. The owner has not said where artifacts go.
- An asyncio client. `grpc.aio` would be a second client to keep in step; it can follow if a caller needs it.
- Node.js, paused by the owner.
