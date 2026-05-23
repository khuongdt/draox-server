use crate::error::ApiError;
use crate::response::ApiResponse;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::response::IntoResponse;

/// GET /api/audit — list recent audit entries
pub async fn list_audit(State(state): State<AppState>) -> impl IntoResponse {
    let entries = state.audit_log.entries();
    let total = entries.len();
    ApiResponse::ok(serde_json::json!({
        "total": total,
        "entries": entries,
    }))
}

/// GET /api/audit/:id — get audit entry by sequence ID
pub async fn get_audit_entry(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse, ApiError> {
    let entry = state
        .audit_log
        .get_by_id(id)
        .ok_or_else(|| ApiError::not_found(format!("audit entry not found: {id}")))?;
    Ok(ApiResponse::ok(entry))
}
