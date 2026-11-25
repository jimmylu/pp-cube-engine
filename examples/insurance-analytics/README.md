# Example: an insurer's analytics portal

A property insurer's analysts open dashboards. Each dashboard question is a
Cube query; the portal's back end turns it into SQL through the kernel and runs
that SQL on the warehouse. This example is that back end, small but whole, and
it runs.

```sh
scripts/vendor-sync.sh                      # once, to build the service image
examples/insurance-analytics/run.sh         # the Java back end
examples/insurance-analytics/run.sh python  # the Python back end
```

The first run builds the service image and pulls Presto, which takes several
minutes; later runs reuse both. It ends with `example: PASSED`, leaves nothing
running and removes the certificate and tokens it made. It needs Docker with
Compose and openssl, and for the back end of your choice a JDK 11 or later
with Maven, or Python 3.9 or later.

## What is in it

| | Stands for |
| --- | --- |
| `model-store/` | Shared storage a publisher writes: `property/models/2026-11/`, one cube or view a file, and a manifest |
| `docker-compose.yaml` | The compile service, started with the arguments of [deploy/kubernetes.yaml](../../deploy/kubernetes.yaml), and Presto 0.296 as the warehouse |
| `warehouse/insurance.sql` | The warehouse's tables: policies, earned premium, claims, reserves, payments, cessions |
| `portal.json` | Four users, their row-level rules, their questions, and the answers worked out by hand |
| `app/` | The back end in Java, on [the Java client](../../clients/java) and the Presto JDBC driver |
| `app-python/` | The same back end in Python, on [the Python client](../../clients/python) and `presto-python-client` |
| `run.sh` | An operator: makes a certificate and tokens, starts both systems, runs the back end, stops everything |

## What happens for one question

1. **Who is asking.** The East region's manager has the rule
   `branches.region_name equals East`. The portal decides that from the user;
   the user never states it.
2. **Compile.** The back end sends the question, the model version, the
   dialect and the rule to the service over TLS with its read token. The
   service reads no data and knows no warehouse.
3. **Execute.** The back end prepares the returned SQL on Presto, binds the
   returned parameters, applies the returned session properties and reads the
   rows by the returned column names. Nothing of the answer is written into
   the SQL text.
4. **Cache.** Rows are kept under the returned fingerprint. When the first
   user's dashboard opens again, every question is compiled again (cheap) and
   answered from the cache: same model content, same question, same rules,
   same key.

The same question, `x_premium_paid_by_line`, is asked by the financial
controller and by the East manager. The SQL differs by the rule, the
fingerprints differ, and the manager sees two lines of business where the
controller sees three.

## What it shows a caller doing

- **Row-level security** through `securityFilter`, on a question over three
  facts: the rule reaches every fact.
- **Bound parameters** and **session properties**: `CompiledQuery.bind` and
  the JDBC driver in Java, `compiled.parameters` handed to `cursor.execute` in
  Python. The paged question needs
  `offset_clause_enabled=true` on Presto; the kernel says so and the back end
  sets it.
- **Rolling windows, a declared granularity, several facts, a view and
  ungrouped rows**: one question each, on a model of nine cubes and two views.
- **Two answers without SQL**, and what to do with each. A ratio of two facts
  written as one measure is a `QUERY` error: ask for both measures and divide.
  Year to date by quarter is `UNSUPPORTED` on Presto: offer it by month. Neither
  is retried.
- **Waiting at start.** The service reads its store when it starts and the
  warehouse answers a moment after it accepts connections; the back end waits
  for both instead of failing.
- **Checking itself.** Every answer is compared with the expected one in
  `portal.json`. A wrong row fails the run.

## What it leaves out

A real portal has users, sessions and a result cache that expires; here the
users are a file and the cache is a map. The warehouse is Presto's memory
connector with a few dozen rows. One replica runs, not three. The certificate
is self-signed for the run and the tokens are random; both are deleted with
`.run/`.

`crates/pp-cube-conformance/tests/example.rs` keeps this example true without
running it: its model must be the property-insurance scenario's, every question
must compile or be refused as `portal.json` says, and the expected answers must
be those of the executed cases.
