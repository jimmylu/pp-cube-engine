# Integration test kit

`ppcube-itest` tests the compile service by hand against real engines: it
compiles Cube queries through the service, executes the SQL on Presto, Doris or
Druid as a caller's back end would, compares the rows with expected answers and
writes a report.

The guide, step by step, is in Chinese:
[docs/integration-testing.zh-CN.md](../../docs/integration-testing.zh-CN.md).

```sh
python3 -m venv .itest && . .itest/bin/activate
pip install --upgrade pip
pip install clients/python tools/integration-kit      # from a checkout
cp tools/integration-kit/itest.example.ini itest.ini  # then describe your environment in it
ppcube-itest env
```

| Command | Does |
| --- | --- |
| `env` | Checks that the service, the model versions and the engines are reachable, and says what they are |
| `meta` | Lists the cubes, views and members of a model version |
| `compile` | A Cube query to SQL: prints the request, the SQL, its parameters, session properties and fingerprint |
| `run` | Compiles, executes on the engine and prints the rows |
| `model` | Writes the benchmark model as a version of a model store, for an engine's schema and data source |
| `load` | Creates the benchmark tables on an engine and loads their rows; `--dry-run` prints the statements |
| `clean` | Drops the benchmark tables, and nothing else |
| `suite` | Runs the built-in benchmark suite, or one of your own, and writes `report.md` and `report.json` |

Run it from a checkout of the repository: the benchmark model, data and
questions are those of [examples/insurance-analytics](../../examples/insurance-analytics),
read from there. `itest.ini` holds no secret: a password is named by the
environment variable that holds it, a token by its file.

What it changes on an engine is limited to the nine tables named `ins_*`, with
the configured prefix, in the configured schema. It creates the schema if it
is missing and never drops it.

The tests of the kit itself:
`python -m unittest discover -s tools/integration-kit/tests`.
