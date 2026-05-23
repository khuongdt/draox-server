use crate::error::ApiError;
use crate::response::ApiResponse;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use serde::Serialize;
use server_core::SessionId;

#[derive(Serialize)]
pub struct SessionListResponse {
    pub total: usize,
    pub sessions: Vec<SessionSummary>,
}

#[derive(Serialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub client_id: String,
    pub connection_count: usize,
    pub created_at: String,
}

/// GET /api/sessions
pub async fn list_sessions(State(state): State<AppState>) -> impl IntoResponse {
    let sessions = state.session_manager.sessions_list();
    let total = sessions.len();
    let summaries: Vec<SessionSummary> = sessions
        .into_iter()
        .map(|s| SessionSummary {
            session_id: s.session_id.to_string(),
            client_id: s.client_id.to_string(),
            connection_count: s.connection_count,
            created_at: s.created_at.to_rfc3339(),
        })
        .collect();

    ApiResponse::ok(SessionListResponse {
        total,
        sessions: summaries,
    })
}

/// DELETE /api/sessions/:id
pub async fn destroy_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let session_id = SessionId::from_str(&id);

    // Check if session exists
    state
        .session_manager
        .get_session(&session_id)
        .ok_or_else(|| ApiError::not_found(format!("session not found: {id}")))?;

    state
        .session_manager
        .destroy_session(&session_id, "admin API request");

    Ok(ApiResponse::<()>::message("session destroyed"))
}

/// GET /api/sessions/:id — get session details
pub async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let session_id = SessionId::from_str(&id);
    let session = state
        .session_manager
        .get_session(&session_id)
        .ok_or_else(|| ApiError::not_found(format!("session not found: {id}")))?;
    let summary = SessionSummary {
        session_id: session.session_id.to_string(),
        client_id: session.client_id.to_string(),
        connection_count: session.connection_count(),
        created_at: session.created_at.to_rfc3339(),
    };
    drop(session); // Drop the DashMap guard
    Ok(ApiResponse::ok(summary))
}

/// POST /api/sessions/:id/drain — start draining a session
pub async fn drain_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let session_id = SessionId::from_str(&id);
    state
        .session_manager
        .drain_session(&session_id)
        .map_err(ApiError::from)?;
    Ok(ApiResponse::<()>::message("session drain started"))
}

/// GET /api/sessions/:id/metrics — get session metrics
pub async fn session_metrics(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let session_id = SessionId::from_str(&id);
    let metrics = state
        .session_manager
        .get_metrics(&session_id)
        .ok_or_else(|| ApiError::not_found(format!("session not found: {id}")))?;
    Ok(ApiResponse::ok(metrics))
}
