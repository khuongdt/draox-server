use opentelemetry::{
    global,
    metrics::{Counter, Gauge, Histogram, Meter},
};
use std::sync::OnceLock;

/// Server-wide OpenTelemetry metrics instruments.
/// Lazily initialised on first access; safe to call from any task.
pub struct ServerMetrics {
    pub active_connections: Gauge<i64>,
    pub total_connections: Counter<u64>,
    pub bytes_received: Counter<u64>,
    pub bytes_sent: Counter<u64>,
    pub request_duration_ms: Histogram<f64>,
    pub plugin_calls: Counter<u64>,
    pub job_queue_depth: Gauge<i64>,
}

static METRICS: OnceLock<ServerMetrics> = OnceLock::new();

impl ServerMetrics {
    fn build(meter: &Meter) -> Self {
        Self {
            active_connections: meter
                .i64_gauge("draox.connections.active")
                .with_description("Number of currently open connections")
                .build(),
            total_connections: meter
                .u64_counter("draox.connections.total")
                .with_description("Cumulative accepted connections since startup")
                .build(),
            bytes_received: meter
                .u64_counter("draox.bytes.received")
                .with_description("Total bytes received across all connections")
                .build(),
            bytes_sent: meter
                .u64_counter("draox.bytes.sent")
                .with_description("Total bytes sent across all connections")
                .build(),
            request_duration_ms: meter
                .f64_histogram("draox.http.request.duration_ms")
                .with_description("Admin API request duration in milliseconds")
                .build(),
            plugin_calls: meter
                .u64_counter("draox.plugin.calls")
                .with_description("Total plugin IPC / hook invocations")
                .build(),
            job_queue_depth: meter
                .i64_gauge("draox.jobs.queue_depth")
                .with_description("Current depth of the background job queue")
                .build(),
        }
    }

    pub fn global() -> &'static ServerMetrics {
        METRICS.get_or_init(|| {
            let meter = global::meter("draox-server");
            Self::build(&meter)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_global_init() {
        let m = ServerMetrics::global();
        m.total_connections.add(1, &[]);
        m.active_connections.record(5, &[]);
        // No panic — instruments are valid even without a real exporter.
    }
}
