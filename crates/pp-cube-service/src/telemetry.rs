//! Request metrics and structured log events for a host to expose.
//!
//! Nothing here records SQL, parameter values, filters, model content or
//! credentials: only which operation ran, for which version, how it ended and
//! how long it took.
use std::{
    collections::BTreeMap,
    fmt::Write as _,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/// Upper bounds of the duration histogram, in seconds.
const BUCKETS: [f64; 10] = [0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 1.0, 5.0];
/// Caller-chosen text is cut to this many characters before it is logged.
const FIELD_LIMIT: usize = 128;

#[derive(Debug, Default, Clone, Copy)]
struct Histogram {
    buckets: [u64; BUCKETS.len()],
    count: u64,
    sum: f64,
}

/// Counters a host renders in the Prometheus text format. Labels come from
/// fixed sets (operation and outcome), never from request content.
#[derive(Debug, Default)]
pub struct Metrics {
    requests: Mutex<BTreeMap<(&'static str, String), u64>>,
    durations: Mutex<BTreeMap<&'static str, Histogram>>,
    in_flight: AtomicI64,
}

impl Metrics {
    pub(crate) fn started(&self) {
        self.in_flight.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn finished(&self, rpc: &'static str, outcome: &str, elapsed: Duration) {
        self.in_flight.fetch_sub(1, Ordering::Relaxed);
        *self
            .requests
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entry((rpc, outcome.to_owned()))
            .or_default() += 1;
        let seconds = elapsed.as_secs_f64();
        let mut durations = self.durations.lock().unwrap_or_else(|e| e.into_inner());
        let histogram = durations.entry(rpc).or_default();
        for (bucket, bound) in histogram.buckets.iter_mut().zip(BUCKETS) {
            if seconds <= bound {
                *bucket += 1;
            }
        }
        histogram.count += 1;
        histogram.sum += seconds;
    }

    /// How many requests ended with `outcome` for `rpc`.
    pub fn requests(&self, rpc: &str, outcome: &str) -> u64 {
        self.requests
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|((name, ended), _)| *name == rpc && ended == outcome)
            .map(|(_, count)| *count)
            .sum()
    }

    /// The Prometheus text exposition. `model_versions` is the number available now.
    pub fn render(&self, model_versions: usize) -> String {
        let mut text = String::new();
        let _ = writeln!(text, "# TYPE ppcube_requests_total counter");
        for ((rpc, outcome), count) in self
            .requests
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
        {
            let _ = writeln!(
                text,
                "ppcube_requests_total{{rpc=\"{rpc}\",outcome=\"{outcome}\"}} {count}"
            );
        }
        let _ = writeln!(text, "# TYPE ppcube_request_duration_seconds histogram");
        for (rpc, histogram) in self
            .durations
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
        {
            for (bound, count) in BUCKETS.iter().zip(histogram.buckets) {
                let _ = writeln!(
                    text,
                    "ppcube_request_duration_seconds_bucket{{rpc=\"{rpc}\",le=\"{bound}\"}} {count}"
                );
            }
            let _ = writeln!(
                text,
                "ppcube_request_duration_seconds_bucket{{rpc=\"{rpc}\",le=\"+Inf\"}} {}",
                histogram.count
            );
            let _ = writeln!(
                text,
                "ppcube_request_duration_seconds_sum{{rpc=\"{rpc}\"}} {}",
                histogram.sum
            );
            let _ = writeln!(
                text,
                "ppcube_request_duration_seconds_count{{rpc=\"{rpc}\"}} {}",
                histogram.count
            );
        }
        let _ = writeln!(text, "# TYPE ppcube_requests_in_flight gauge");
        let _ = writeln!(
            text,
            "ppcube_requests_in_flight {}",
            self.in_flight.load(Ordering::Relaxed)
        );
        let _ = writeln!(text, "# TYPE ppcube_model_versions gauge");
        let _ = writeln!(text, "ppcube_model_versions {model_versions}");
        let _ = writeln!(text, "# TYPE ppcube_build_info gauge");
        let _ = writeln!(
            text,
            "ppcube_build_info{{kernel_version=\"{}\",upstream_pin=\"{}\"}} 1",
            pp_cube_compile::KERNEL_VERSION,
            pp_cube_compile::UPSTREAM_PIN
        );
        text
    }
}

/// One finished request. Optional fields are omitted from the log line.
#[derive(Debug, Clone, Default)]
pub struct Event {
    pub rpc: &'static str,
    /// `ok`, a compile failure kind, or a gRPC status name.
    pub outcome: String,
    pub duration: Duration,
    pub request_id: Option<String>,
    pub model_version: Option<String>,
    pub dialect: Option<String>,
    /// Hex of the result fingerprint, for correlating with a caller's cache.
    pub fingerprint: Option<String>,
}

fn clipped(value: &Option<String>) -> Option<String> {
    value
        .as_ref()
        .filter(|text| !text.is_empty())
        .map(|text| text.chars().take(FIELD_LIMIT).collect())
}

impl Event {
    /// A single JSON line. Caller-chosen text is length-limited and JSON-escaped,
    /// so it cannot forge or split a line.
    pub fn json(&self) -> String {
        let level = match self.outcome.as_str() {
            "ok" | "model" | "query" | "unsupported" => "info",
            "internal" | "Internal" | "Unavailable" | "DeadlineExceeded" => "error",
            _ => "warn",
        };
        let mut line = serde_json::Map::new();
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_millis() as u64);
        line.insert("ts_ms".into(), ts.into());
        line.insert("level".into(), level.into());
        line.insert("event".into(), "request".into());
        line.insert("rpc".into(), self.rpc.into());
        line.insert("outcome".into(), self.outcome.clone().into());
        line.insert(
            "duration_ms".into(),
            ((self.duration.as_secs_f64() * 1e6).round() / 1e3).into(),
        );
        for (name, value) in [
            ("request_id", &self.request_id),
            ("model_version", &self.model_version),
            ("dialect", &self.dialect),
            ("fingerprint", &self.fingerprint),
        ] {
            if let Some(value) = clipped(value) {
                line.insert(name.into(), value.into());
            }
        }
        serde_json::Value::Object(line).to_string()
    }
}

type Sink = Arc<dyn Fn(String) + Send + Sync>;

/// Where a service reports what it did. Metrics are always kept; log lines go
/// to the sink when there is one.
#[derive(Clone, Default)]
pub struct Telemetry {
    pub metrics: Arc<Metrics>,
    sink: Option<Sink>,
}

impl Telemetry {
    /// Send one JSON line per finished request to `sink`.
    pub fn with_log(mut self, sink: impl Fn(String) + Send + Sync + 'static) -> Self {
        self.sink = Some(Arc::new(sink));
        self
    }

    pub(crate) fn finish(&self, event: Event) {
        self.metrics
            .finished(event.rpc, &event.outcome, event.duration);
        if let Some(sink) = &self.sink {
            sink(event.json());
        }
    }
}
