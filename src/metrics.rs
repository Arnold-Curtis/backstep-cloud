//! Prometheus metrics for gRPC request observability.
//!
//! Tracks RED metrics (Rate, Errors, Duration) per RPC method,
//! exposed via a side HTTP server on the configured admin port.

use metrics::{counter, histogram};
use metrics_exporter_prometheus::PrometheusBuilder;
use std::net::SocketAddr;
use tokio::task::JoinHandle;

/// Start a Prometheus metrics HTTP server on the given port.
///
/// Returns a handle that completes when the server exits.
/// The global recorder is installed — subsequent calls to
/// `metrics::counter!()` etc. will be collected.
pub fn start_metrics_server(port: u16) -> JoinHandle<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let (recorder, exporter) = PrometheusBuilder::new()
        .with_http_listener(addr)
        .build()
        .expect("failed to build Prometheus exporter");

    metrics::set_global_recorder(recorder)
        .expect("global metrics recorder already set");

    tracing::info!(
        port,
        "Prometheus metrics endpoint listening on /metrics"
    );

    tokio::spawn(async move {
        if let Err(e) = exporter.await {
            tracing::error!(error = %e, "Prometheus exporter crashed");
        }
    })
}

// ── RED Metrics ────────────────────────────────────────────

/// Record a successful gRPC request for an RPC method.
pub fn record_request(method: &str) {
    counter!("backstep_cloud_requests_total", "method" => method.to_string()).increment(1);
}

/// Record a failed gRPC request for an RPC method.
pub fn record_error(method: &str, status_code: &str) {
    counter!(
        "backstep_cloud_errors_total",
        "method" => method.to_string(),
        "status" => status_code.to_string()
    )
    .increment(1);
}

/// Record the duration (in seconds) of a gRPC request.
pub fn record_duration(method: &str, seconds: f64) {
    histogram!(
        "backstep_cloud_request_duration_seconds",
        "method" => method.to_string()
    )
    .record(seconds);
}
