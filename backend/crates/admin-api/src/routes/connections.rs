use crate::error::ApiError;
use crate::response::ApiResponse;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use server_core::ConnectionId;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
pub struct ConnectionListResponse {
    pub total: usize,
    pub connections: Vec<ConnectionSummary>,
}

#[derive(Serialize)]
pub struct ConnectionSummary {
    pub id: String,
    pub protocol: String,
    pub remote_addr: String,
    pub state: String,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

/// GET /api/connections
pub async fn list_connections(State(state): State<AppState>) -> impl IntoResponse {
    let conns = state.connection_tracker.connections();
    let total = conns.len();
    let connections: Vec<ConnectionSummary> = conns
        .into_iter()
        .map(|c| ConnectionSummary {
            id: c.id.to_string(),
            protocol: c.protocol.to_string(),
            remote_addr: c.remote_addr.to_string(),
            state: format!("{:?}", c.state),
            bytes_sent: c.bytes_sent,
            bytes_received: c.bytes_received,
        })
        .collect();

    ApiResponse::ok(ConnectionListResponse {
        total,
        connections,
    })
}

/// GET /api/connections/:id
pub async fn get_connection(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let conn_id = ConnectionId::from_str(&id);
    let info = state
        .connection_tracker
        .get(&conn_id)
        .ok_or_else(|| ApiError::not_found(format!("connection not found: {id}")))?;

    Ok(ApiResponse::ok(ConnectionSummary {
        id: info.id.to_string(),
        protocol: info.protocol.to_string(),
        remote_addr: info.remote_addr.to_string(),
        state: format!("{:?}", info.state),
        bytes_sent: info.bytes_sent,
        bytes_received: info.bytes_received,
    }))
}

/// DELETE /api/connections/:id — disconnect a connection
pub async fn disconnect_connection(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let conn_id = ConnectionId::from_str(&id);
    // Check connection exists
    state
        .connection_tracker
        .get(&conn_id)
        .ok_or_else(|| ApiError::not_found(format!("connection not found: {id}")))?;
    // Remove from tracker
    state.connection_tracker.unregister(&conn_id);
    // Unbind from session
    state.session_manager.unbind_connection(&conn_id);
    Ok(ApiResponse::<()>::message("connection disconnected"))
}

/// GET /api/connections/stats — connection statistics
#[derive(Serialize)]
pub struct ConnectionStatsResponse {
    pub total_connections: usize,
    pub by_protocol: HashMap<String, usize>,
}

pub async fn connection_stats(State(state): State<AppState>) -> impl IntoResponse {
    let conns = state.connection_tracker.connections();
    let mut by_protocol: HashMap<String, usize> = HashMap::new();
    for c in &conns {
        *by_protocol.entry(c.protocol.to_string()).or_insert(0) += 1;
    }
    ApiResponse::ok(ConnectionStatsResponse {
        total_connections: conns.len(),
        by_protocol,
    })
}
