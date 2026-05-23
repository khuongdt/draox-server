use crate::response::ApiResponse;
use crate::state::AppState;
use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use chrono::{DateTime, Utc};
use serde::Serialize;

/// JSON shape returned by `GET /api/metrics`.
///
/// Field names are intentionally kept short (`bytes_sent` / `bytes_received`)
/// to match the `API.MetricsSnapshot` TypeScript interface used by the frontend.
/// `timestamp` is an ISO-8601 string so the JS `Date` constructor can parse it.
#[derive(Serialize)]
pub struct MetricsResponse {
    /// ISO-8601 UTC timestamp of when the snapshot was taken.
    pub timestamp: DateTime<Utc>,
    pub connections_active: i64,
    pub connections_total: u64,
    /// Renamed from `bytes_received_total` to match the frontend interface.
    #[serde(rename = "bytes_received")]
    pub bytes_received_total: u64,
    /// Renamed from `bytes_sent_total` to match the frontend interface.
    #[serde(rename = "bytes_sent")]
    pub bytes_sent_total: u64,
    pub requests_total: u64,
    pub errors_total: u64,
}

/// GET /api/metrics — returns a single point-in-time snapshot of all server metrics.
pub async fn get_metrics(State(state): State<AppState>) -> impl IntoResponse {
    let snapshot = state.metrics.snapshot();

    ApiResponse::ok(MetricsResponse {
        timestamp: snapshot.timestamp,
        connections_active: snapshot.connections_active,
        connections_total: snapshot.connections_total,
        bytes_received_total: snapshot.bytes_received_total,
        bytes_sent_total: snapshot.bytes_sent_total,
        requests_total: snapshot.requests_total,
        errors_total: snapshot.errors_total,
    })
}

/// GET /api/metrics/activity — activity log summary
pub async fn activity_summary(State(state): State<AppState>) -> impl IntoResponse {
    let filter = activity_log::LogFilter {
        limit: Some(100),
        ..Default::default()
    };
    let recent = state.activity_log.query(&filter);
    ApiResponse::ok(serde_json::json!({
        "total_entries": state.activity_log.count(),
        "recent_count": recent.len(),
    }))
}

/// GET /api/metrics/prometheus — Prometheus text format.
pub async fn get_metrics_prometheus(State(state): State<AppState>) -> impl IntoResponse {
    let s = state.metrics.snapshot();
    let text = format!(
        "# HELP draox_connections_active Current active connections\n\
         # TYPE draox_connections_active gauge\n\
         draox_connections_active {}\n\
         # HELP draox_connections_total Total connections accepted\n\
         # TYPE draox_connections_total counter\n\
         draox_connections_total {}\n\
         # HELP draox_bytes_received_total Total bytes received\n\
         # TYPE draox_bytes_received_total counter\n\
         draox_bytes_received_total {}\n\
         # HELP draox_bytes_sent_total Total bytes sent\n\
         # TYPE draox_bytes_sent_total counter\n\
         draox_bytes_sent_total {}\n\
         # HELP draox_requests_total Total requests processed\n\
         # TYPE draox_requests_total counter\n\
         draox_requests_total {}\n\
         # HELP draox_errors_total Total errors\n\
         # TYPE draox_errors_total counter\n\
         draox_errors_total {}\n\
         # HELP draox_sessions_active Current active sessions\n\
         # TYPE draox_sessions_active gauge\n\
         draox_sessions_active {}\n\
         # HELP draox_plugins_registered Total registered plugins\n\
         # TYPE draox_plugins_registered gauge\n\
         draox_plugins_registered {}\n",
        s.connections_active,
        s.connections_total,
        s.bytes_received_total,
        s.bytes_sent_total,
        s.requests_total,
        s.errors_total,
        state.session_manager.session_count(),
        state.plugin_registry.count(),
    );

    (
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        text,
    )
}
