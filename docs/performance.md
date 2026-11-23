# Performance

How long a compilation takes, how it was measured, and what to do about it.

## Measure

```sh
cargo run --release -p pp-cube-conformance --bin compile-bench -- [cubes] [iterations] [threads]
```

The tool compiles the forty shared execution cases for Presto and Doris against
the shared model padded to `cubes` cubes (each copy has fifteen members), and
prints percentiles per phase and the throughput on one and on `threads` threads.
Numbers depend on the machine: compare runs on the same one.

## Measured on 2026-11-23

Intel Core i9-9880H (16 logical cores), macOS, rustc 1.98.1, release build,
1600 compilations per row. Latency in milliseconds, single thread.

| Model | Validate | Digest | Compile, validating each time (p50 / p99) | Compile, prepared model (p50 / p99) |
| --- | --- | --- | --- | --- |
| 4 cubes, 21 members | 0.40 | 0.07 | 0.68 / 1.15 | 0.26 / 0.51 |
| 50 cubes, 711 members | 9.4 | 1.5 | 11.5 / 15.8 | 0.64 / 1.00 |
| 200 cubes, 2961 members | 40.6 | 7.7 | 48.7 / 60.2 | 1.89 / 2.67 |

Throughput with a prepared model, compilations per second:

| Model | 1 thread | 4 threads |
| --- | --- | --- |
| 4 cubes | 3523 | 12289 |
| 50 cubes | 1544 | 5434 |
| 200 cubes | 527 | 1909 |

## What it shows

- Validating and digesting the model dominated every compilation and grew with
  the model: at 50 cubes they were 95% of the time. `PreparedModel` does both
  once; the service prepares a version when it is published, so a compilation
  no longer pays for them. The result is identical, fingerprint included.
- Compilations share only the immutable model, so throughput scales with
  threads (about 3.5 times on 4 threads here). Size the service with `--workers`.
- What remains still grows with the model, roughly 10 microseconds per cube.
  It has not been profiled; structures built per compilation from the whole
  model are the likely cause and could be built once per version.

These are compilation figures only. They do not include the network, TLS or
the queue, and nothing here measures the SQL's execution on an engine.
