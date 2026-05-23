use crate::response::ApiResponse;
use crate::state::AppState;
use axum::extract::State;
use axum::response::IntoResponse;
use serde::Serialize;

#[derive(Serialize)]
pub struct ConfigSummaryResponse {
    pub admin_bind_addr: String,
    pub session_max_connections: usize,
    pub session_grace_period_secs: u64,
}

/// GET /api/config — return current server configuration summary
pub async fn get_config(State(state): State<AppState>) -> impl IntoResponse {
    let session_config = state.session_manager.config();
    ApiResponse::ok(ConfigSummaryResponse {
        admin_bind_addr: "127.0.0.1:9100".to_string(),
        session_max_connections: session_config.max_connections_per_session,
        session_grace_period_secs: session_config.grace_period_secs,
    })
}

/// POST /api/config/reload — placeholder for hot-reload trigger
pub async fn reload_config() -> impl IntoResponse {
    ApiResponse::<()>::message("config reload triggered (hot-reload not yet connected)")
}
