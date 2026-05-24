use crate::error::ApiError;
use crate::response::ApiResponse;
use crate::state::AppState;
use activity_log::{AuditAction, AuditEntry};
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use serde::Serialize;

// Response DTO — maps backend AuditEntry to frontend-expected shape.
#[derive(Serialize)]
struct AuditEntryDto {
    id: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    actor: String,
    action: String,
    target: String,
    severity: &'static str,
    details: Option<serde_json::Value>,
    source_ip: Option<String>,
    trace_id: Option<String>,
}

impl From<AuditEntry> for AuditEntryDto {
    fn from(e: AuditEntry) -> Self {
        let severity = action_severity(&e.action);
        // AuditAction serializes to snake_case (e.g. "login_success")
        let action = serde_json::to_value(&e.action)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| format!("{:?}", e.action));
        Self {
            id: e.sequence_id.to_string(),
            timestamp: e.timestamp,
            actor: e.actor,
            action,
            target: e.resource,
            severity,
            details: e.details,
            source_ip: e.source_ip,
            trace_id: e.trace_id,
        }
    }
}

fn action_severity(action: &AuditAction) -> &'static str {
    match action {
        AuditAction::IpBanned
        | AuditAction::BlacklistUpdated
        | AuditAction::LoginFailed
        | AuditAction::PluginUninstalled => "high",
        AuditAction::ConfigUpdated
        | AuditAction::PluginActivated
        | AuditAction::PluginDeactivated
        | AuditAction::PluginInstalled
        | AuditAction::ApiKeyCreated
        | AuditAction::ApiKeyRevoked => "medium",
        _ => "low",
    }
}

/// GET /api/audit — list all audit entries (newest first)
pub async fn list_audit(State(state): State<AppState>) -> impl IntoResponse {
    let entries: Vec<AuditEntryDto> = state
        .audit_log
        .entries()
        .into_iter()
        .map(AuditEntryDto::from)
        .collect();
    let total = entries.len();
    ApiResponse::ok(serde_json::json!({ "total": total, "entries": entries }))
}

/// GET /api/audit/{id} — get a single audit entry by sequence ID
pub async fn get_audit_entry(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse, ApiError> {
    let entry = state
        .audit_log
        .get_by_id(id)
        .ok_or_else(|| ApiError::not_found(format!("audit entry not found: {id}")))?;
    Ok(ApiResponse::ok(AuditEntryDto::from(entry)))
}
