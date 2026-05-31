use crate::error::ApiError;
use crate::response::ApiResponse;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use plugin_sdk::traits::{PluginHealth, PluginState};
use server_core::PluginId;
use serde::Serialize;

#[derive(Serialize)]
pub struct PluginListResponse {
    pub total: usize,
    pub plugins: Vec<PluginSummary>,
}

#[derive(Serialize)]
pub struct PluginSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    /// "builtin" or "wasm". Determined heuristically — see `plugin_type_of`.
    pub plugin_type: &'static str,
    pub state: PluginState,
    pub registered_at: String,
    pub activated_at: Option<String>,
}

/// Built-in plugins are registered through `register_builtin` and have IDs
/// from the well-known `io.draox.*` namespace. Anything else is assumed to
/// be a marketplace / WASM plugin until the registry surfaces explicit type
/// metadata (tracked separately).
fn plugin_type_of(id: &str) -> &'static str {
    if id.starts_with("io.draox.") { "builtin" } else { "wasm" }
}

/// GET /api/plugins
pub async fn list_plugins(State(state): State<AppState>) -> impl IntoResponse {
    // `registry.list()` is sync and returns IDs + state quickly; `get_info`
    // is async and locks each plugin briefly to read `name` / `version`
    // from the trait. We pay that lock cost once per request — plugin lists
    // are small (single digits in practice).
    let basic = state.plugin_registry.list();
    let mut summaries: Vec<PluginSummary> = Vec::with_capacity(basic.len());
    for entry in basic {
        let (name, version) = match state.plugin_registry.get_info(&entry.id).await {
            Ok(info) => (info.name, info.version),
            Err(_)   => (String::new(), String::new()),
        };
        summaries.push(PluginSummary {
            id: entry.id.to_string(),
            plugin_type: plugin_type_of(entry.id.as_str()),
            name,
            version,
            state: entry.state,
            registered_at: entry.registered_at.to_rfc3339(),
            activated_at: entry.activated_at.map(|t| t.to_rfc3339()),
        });
    }
    let total = summaries.len();

    ApiResponse::ok(PluginListResponse {
        total,
        plugins: summaries,
    })
}

/// GET /api/plugins/:id
pub async fn get_plugin(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let plugin_id = PluginId::from_str(&id);
    let info = state
        .plugin_registry
        .get_info(&plugin_id)
        .await
        .map_err(ApiError::from)?;

    Ok(ApiResponse::ok(PluginSummary {
        id: info.id.to_string(),
        plugin_type: plugin_type_of(info.id.as_str()),
        name: info.name,
        version: info.version,
        state: info.state,
        registered_at: info.registered_at.to_rfc3339(),
        activated_at: info.activated_at.map(|t| t.to_rfc3339()),
    }))
}

/// POST /api/plugins/:id/activate
pub async fn activate_plugin(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let plugin_id = PluginId::from_str(&id);
    state
        .plugin_registry
        .activate(&plugin_id)
        .await
        .map_err(ApiError::from)?;
    Ok(ApiResponse::<()>::message("plugin activated"))
}

/// POST /api/plugins/:id/deactivate
pub async fn deactivate_plugin(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let plugin_id = PluginId::from_str(&id);
    state
        .plugin_registry
        .deactivate(&plugin_id)
        .await
        .map_err(ApiError::from)?;
    Ok(ApiResponse::<()>::message("plugin deactivated"))
}

/// POST /api/plugins/:id/enable
pub async fn enable_plugin(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let plugin_id = PluginId::from_str(&id);
    state
        .plugin_registry
        .enable(&plugin_id)
        .await
        .map_err(ApiError::from)?;
    Ok(ApiResponse::<()>::message("plugin enabled"))
}

/// POST /api/plugins/:id/disable
pub async fn disable_plugin(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let plugin_id = PluginId::from_str(&id);
    state
        .plugin_registry
        .disable(&plugin_id)
        .await
        .map_err(ApiError::from)?;
    Ok(ApiResponse::<()>::message("plugin disabled"))
}

/// POST /api/plugins/:id/restart — restart a plugin
pub async fn restart_plugin(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let plugin_id = PluginId::from_str(&id);
    state
        .plugin_registry
        .restart(&plugin_id)
        .await
        .map_err(ApiError::from)?;
    Ok(ApiResponse::<()>::message("plugin restarted"))
}

/// GET /api/plugins/:id/health — plugin health check
pub async fn plugin_health(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let plugin_id = PluginId::from_str(&id);
    let health = state
        .plugin_registry
        .health_check(&plugin_id)
        .await
        .map_err(ApiError::from)?;
    let message = match &health {
        PluginHealth::Healthy => "healthy".to_string(),
        PluginHealth::Degraded { reason } => format!("degraded: {reason}"),
        PluginHealth::Unhealthy { reason } => format!("unhealthy: {reason}"),
    };
    Ok(ApiResponse::ok(serde_json::json!({
        "plugin_id": id,
        "healthy": health.is_healthy(),
        "message": message,
    })))
}
