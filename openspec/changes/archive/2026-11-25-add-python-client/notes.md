# Notes

## Done

- `clients/python`, module `ppcube`: the client and nine unit tests against a stand-in gRPC server.
- `examples/insurance-analytics/run.sh python`: the example with a Python back end, end to end.
- B1-CLIENT-003 ties both to the contract and to the Java twin under T0.

## Evidence

- Local, real containers (service image built from this checkout, Presto 0.296), Python 3.9.6: `23 questions asked: 20 answered as expected, 3 refused as expected, 5 served from the cache` and `example: PASSED`, 51 s with the image already built.
- The Python and the Java back end report the same fingerprints for the same questions under the same rules, for example the paged question and the East manager's.
- `python -m unittest discover -s clients/python/tests`: 9 tests pass.
- Not run in CI: the Python tests and the Python back end. Only the Rust test runs there.

## Learned

- The pip that a fresh `venv` of Python 3.9 carries (21.2) copies a package to a temporary directory before building it, away from the contract its stubs are generated from. The first two runs of the example failed there. The build now falls back to stubs that are already generated and says what to do otherwise; `run.sh` upgrades pip in its own environment; the README states the pip version.
- The second of those failures was mine: an edit to `run.sh` did not apply and was not checked before the run. Checked by reading the file since.
- `protoc` writes an absolute import between the two generated Python modules; inside a package it has to be relative.
- Presto's Python driver prepares the statement and passes values in `EXECUTE … USING`, typed by their Python type, so the kernel's `float` parameters arrive as `DOUBLE`.

## Open

- CI for Python: a harness change.
- Where client artifacts are published; the Maven group and this distribution's name wait for that.
- An asyncio client, if a caller needs one.
