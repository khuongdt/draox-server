// NIE 2026-04-14 BATCH-4B WebSocket stream endpoints for admin dashboard

use crate::state::AppState;
use activity_log::MetricsCollector;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use chrono::{DateTime, Utc};
use serde::Serialize;
use server_core::event::ServerEvent;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, warn};

/// Wire shape sent over `/ws/metrics`.
///
/// Mirrors `API.MetricsSnapshot` in the frontend — field names must stay in sync.
/// `bytes_sent_total` / `bytes_received_total` are intentionally renamed so that
/// every data source (HTTP + WS) delivers the same JSON keys.
#[derive(Serialize)]
struct WsMetricsFrame {
    timestamp: DateTime<Utc>,
    connections_active: i64,
    connections_total: u64,
    #[serde(rename = "bytes_sent")]
    bytes_sent_total: u64,
    #[serde(rename = "bytes_received")]
    bytes_received_total: u64,
    requests_total: u64,
    errors_total: u64,
}

// ────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────

/// Pump events from a broadcast receiver into a WebSocket.
///
/// Runs until the client disconnects, the event bus closes, or a
/// serialization error terminates the stream. Lagged events are
/// silently skipped with a warning.
async fn pump_events(mut socket: WebSocket, mut rx: broadcast::Receiver<Arc<ServerEvent>>) {
    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(event) => {
                        let json = match serde_json::to_string(&*event) {
                            Ok(j) => j,
                            Err(e) => {
                                warn!("failed to serialize event: {e}");
                                continue;
                            }
                        };
                        if socket.send(Message::Text(json.into())).await.is_err() {
                            debug!("WebSocket client disconnected");
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("WebSocket stream lagged, skipped {n} events");
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        debug!("Event bus closed");
                        break;
                    }
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => {
                        debug!("WebSocket client closed");
                        break;
                    }
                    _ => {} // ignore other client messages
                }
            }
        }
    }
}

/// Periodically send metrics snapshots via WebSocket.
async fn pump_metrics(
    mut socket: WebSocket,
    metrics: Arc<MetricsCollector>,
    interval_secs: u64,
) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let s = metrics.snapshot();
                // Serialize into WsMetricsFrame so field names match API.MetricsSnapshot
                let frame = WsMetricsFrame {
                    timestamp: s.timestamp,
                    connections_active: s.connections_active,
                    connections_total: s.connections_total,
                    bytes_sent_total: s.bytes_sent_total,
                    bytes_received_total: s.bytes_received_total,
                    requests_total: s.requests_total,
                    errors_total: s.errors_total,
                };
                let json = match serde_json::to_string(&frame) {
                    Ok(j) => j,
                    Err(e) => {
                        warn!("failed to serialize metrics: {e}");
                        continue;
                    }
                };
                if socket.send(Message::Text(json.into())).await.is_err() {
                    debug!("Metrics WebSocket client disconnected");
                    break;
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}

// ────────────────────────────────────────────────────────
// Handlers
// ────────────────────────────────────────────────────────

/// GET /ws/events — stream all server events.
pub async fn ws_events(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let rx = state.event_bus.subscribe_all();
    ws.on_upgrade(move |socket| pump_events(socket, rx))
}

/// GET /ws/connections — stream connection events only.
pub async fn ws_connections(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let rx = state.event_bus.subscribe_topic("connection");
    ws.on_upgrade(move |socket| pump_events(socket, rx))
}

/// GET /ws/plugins — stream plugin events only.
pub async fn ws_plugins(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let rx = state.event_bus.subscribe_topic("plugin");
    ws.on_upgrade(move |socket| pump_events(socket, rx))
}

/// GET /ws/guard — stream traffic guard events.
pub async fn ws_guard(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let rx = state.event_bus.subscribe_topic("guard");
    ws.on_upgrade(move |socket| pump_events(socket, rx))
}

/// GET /ws/metrics — stream periodic metrics snapshots (every 5 seconds).
pub async fn ws_metrics(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let metrics = Arc::clone(&state.metrics);
    ws.on_upgrade(move |socket| pump_metrics(socket, metrics, 5))
}

// ────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    #[test]
    fn test_ws_stream_module_compiles() {
        // If this test runs, the module compiles correctly.
        // Actual WebSocket behavior is tested via integration tests
        // that spawn a full server with WebSocket handshake.
    }
}
