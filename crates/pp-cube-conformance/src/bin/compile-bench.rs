//! Measures compilation latency and throughput: `compile-bench [cubes] [iterations] [threads]`.
//!
//! Compiles the shared execution cases for Presto and Doris against a model
//! padded to `cubes` cubes, prints percentiles per phase, then compiles on
//! `threads` threads at once to show how throughput scales. Numbers depend on
//! the machine; compare runs on the same one. Build with `--release`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use pp_cube_compile::{compile, CompileContext, DialectId};

fn percentile(sorted: &[Duration], p: f64) -> f64 {
    let index = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[index].as_secs_f64() * 1e3
}

fn report(name: &str, mut samples: Vec<Duration>) {
    samples.sort();
    let total: Duration = samples.iter().sum();
    println!(
        "{name:<28} n={:<6} p50={:>8.3}ms p95={:>8.3}ms p99={:>8.3}ms mean={:>8.3}ms",
        samples.len(),
        percentile(&samples, 0.50),
        percentile(&samples, 0.95),
        percentile(&samples, 0.99),
        total.as_secs_f64() * 1e3 / samples.len() as f64,
    );
}

fn main() -> Result<(), String> {
    let mut arguments = std::env::args().skip(1);
    let mut number = |default: usize| -> Result<usize, String> {
        arguments.next().map_or(Ok(default), |text| {
            text.parse()
                .map_err(|_| "usage: compile-bench [cubes] [iterations] [threads]".to_owned())
        })
    };
    let (cubes, iterations, threads) = (number(50)?, number(20)?, number(4)?);
    let model = Arc::new(pp_cube_conformance::padded_model(cubes)?);
    let cases = pp_cube_conformance::bench_cases()?;
    println!(
        "model: {} cubes, {} members; {} cases x 2 dialects x {iterations} iterations",
        model.cubes.len(),
        model
            .cubes
            .iter()
            .map(|c| c.dimensions.len() + c.measures.len())
            .sum::<usize>(),
        cases.len()
    );

    let mut validate = Vec::new();
    let mut digest = Vec::new();
    for _ in 0..iterations {
        let started = Instant::now();
        let issues = pp_cube_model::validate_model(&model);
        validate.push(started.elapsed());
        if !issues.is_empty() {
            return Err("the padded model is not valid".into());
        }
        let started = Instant::now();
        pp_cube_compile::model_digest(&model).map_err(|e| e.to_string())?;
        digest.push(started.elapsed());
    }
    report("validate model", validate);
    report("digest model", digest);

    let prepared = pp_cube_compile::PreparedModel::new(Arc::clone(&model))
        .map_err(|_| "the padded model is not valid".to_owned())?;
    let mut unprepared = Vec::new();
    let mut with_prepared = Vec::new();
    for _ in 0..iterations {
        for dialect in [DialectId::Presto, DialectId::Doris] {
            for (query, timezone, security_filters) in &cases {
                let context = CompileContext {
                    request_id: String::new(),
                    dialect,
                    timezone: timezone.clone(),
                    security_filters: security_filters.clone(),
                };
                let started = Instant::now();
                let first = compile(&model, query, &context).map_err(|e| e.to_string())?;
                unprepared.push(started.elapsed());
                let started = Instant::now();
                let second = prepared
                    .compile(query, &context)
                    .map_err(|e| e.to_string())?;
                with_prepared.push(started.elapsed());
                if first != second {
                    return Err("prepared and unprepared results differ".into());
                }
            }
        }
    }
    report("compile (validates each)", unprepared);
    report("compile (prepared model)", with_prepared);

    // The planner is not thread-safe, but nothing is shared between compilations
    // except the immutable model, so threads should scale until cores run out.
    for count in [1, threads] {
        let started = Instant::now();
        let compiled: usize = std::thread::scope(|scope| {
            let workers: Vec<_> = (0..count)
                .map(|_| {
                    scope.spawn(|| {
                        let mut done = 0;
                        for _ in 0..iterations {
                            for dialect in [DialectId::Presto, DialectId::Doris] {
                                for (query, timezone, security_filters) in &cases {
                                    let context = CompileContext {
                                        request_id: String::new(),
                                        dialect,
                                        timezone: timezone.clone(),
                                        security_filters: security_filters.clone(),
                                    };
                                    if prepared.compile(query, &context).is_ok() {
                                        done += 1;
                                    }
                                }
                            }
                        }
                        done
                    })
                })
                .collect();
            workers.into_iter().map(|w| w.join().unwrap_or(0)).sum()
        });
        let elapsed = started.elapsed().as_secs_f64();
        println!(
            "throughput, {count} thread(s)       {compiled} compilations in {elapsed:.3}s = {:.0}/s",
            compiled as f64 / elapsed
        );
        if count == threads {
            break;
        }
    }
    Ok(())
}
