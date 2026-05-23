use crate::auth::AuthContext;
use crate::error::ApiError;
use crate::response::ApiResponse;
use crate::state::AppState;
use activity_log::AuditAction;
use axum::extract::{Extension, State};
use axum::response::IntoResponse;
use axum::Json;
use chrono::Utc;
use server_config::DraoxConfig;

/// GET /api/config — return full server configuration with sensitive fields redacted
pub async fn get_config(State(state): State<AppState>) -> impl IntoResponse {
    let cfg = state.config.read().unwrap();
    let mut value = serde_json::to_value(&*cfg).unwrap_or_default();
    mask_sensitive(&mut value);
    ApiResponse::ok(value)
}

/// PUT /api/config — update configuration, create backup, write to file, record audit log
pub async fn update_config(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, ApiError> {
    // Restore REDACTED fields from current config before deserializing
    let body = restore_redacted(body, &state);

    // Deserialize and validate
    let new_config: DraoxConfig = serde_json::from_value(body)
        .map_err(|e| ApiError::bad_request(format!("invalid config: {e}")))?;

    // Compute diff for audit log
    let diff = {
        let old = state.config.read().unwrap();
        compute_diff(
            &serde_json::to_value(&*old).unwrap_or_default(),
            &serde_json::to_value(&new_config).unwrap_or_default(),
            String::new(),
        )
    };

    // Create backup of current config file
    let backup_name = create_backup(&state.config_path)
        .map_err(|e| ApiError::internal(format!("backup failed: {e}")))?;

    // Write new config to file
    let toml_str = toml::to_string_pretty(&new_config)
        .map_err(|e| ApiError::internal(format!("serialize failed: {e}")))?;
    std::fs::write(&state.config_path, &toml_str)
        .map_err(|e| ApiError::internal(format!("write failed: {e}")))?;

    // Update in-memory config
    *state.config.write().unwrap() = new_config;

    // Audit log
    let change_count = diff.len();
    state.audit_log.record(
        ctx.identity.as_str(),
        AuditAction::ConfigUpdated,
        "server_config",
        Some(serde_json::json!({ "changes": diff, "backup": backup_name })),
        None,
        None,
    );

    Ok(ApiResponse::<()>::message(format!(
        "Config updated ({change_count} change(s)). Backup saved as '{backup_name}'. Some changes require a server restart to take effect."
    )))
}

/// POST /api/config/reload — placeholder for hot-reload trigger
pub async fn reload_config(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    state.audit_log.record(
        ctx.identity.as_str(),
        AuditAction::ConfigReloaded,
        "server_config",
        None,
        None,
        None,
    );
    ApiResponse::<()>::message("config reload triggered (hot-reload not yet connected)")
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Replace sensitive values in the JSON config with "[REDACTED]" before returning to client.
fn mask_sensitive(value: &mut serde_json::Value) {
    let paths: &[&[&str]] = &[
        &["admin_api", "jwt_secret"],
        &["admin_api", "api_keys"],
        &["storage", "sql", "url"],
        &["storage", "mongodb", "url"],
        &["cache", "redis", "url"],
    ];
    for path in paths {
        if let Some(target) = get_nested_mut(value, path) {
            *target = serde_json::json!("[REDACTED]");
        }
    }
}

/// Before deserializing a client-submitted config, put back the real sensitive values
/// from current in-memory config (so clients cannot accidentally wipe them by submitting "[REDACTED]").
fn restore_redacted(mut body: serde_json::Value, state: &AppState) -> serde_json::Value {
    let cfg = state.config.read().unwrap();
    let live = serde_json::to_value(&*cfg).unwrap_or_default();

    let paths: &[&[&str]] = &[
        &["admin_api", "jwt_secret"],
        &["admin_api", "api_keys"],
        &["storage", "sql", "url"],
        &["storage", "mongodb", "url"],
        &["cache", "redis", "url"],
    ];
    for path in paths {
        if let Some(live_val) = get_nested(&live, path) {
            if let Some(submitted) = get_nested(&body, path) {
                if submitted == &serde_json::json!("[REDACTED]") {
                    if let Some(target) = get_nested_mut(&mut body, path) {
                        *target = live_val.clone();
                    }
                }
            }
        }
    }
    body
}

/// Create a timestamped backup of the config file in config/backups/.
/// Returns the backup filename (not full path).
fn create_backup(config_path: &str) -> std::io::Result<String> {
    let src = std::path::Path::new(config_path);
    if !src.exists() {
        // Nothing to back up yet
        return Ok("(no existing file)".to_string());
    }

    let backup_dir = src
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("backups");
    std::fs::create_dir_all(&backup_dir)?;

    let ts = Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("config_backup_{ts}.toml");
    let dest = backup_dir.join(&filename);
    std::fs::copy(src, &dest)?;
    Ok(filename)
}

/// Recursively diff two JSON values, collecting changed leaf paths.
fn compute_diff(
    old: &serde_json::Value,
    new: &serde_json::Value,
    prefix: String,
) -> Vec<serde_json::Value> {
    let mut changes = Vec::new();
    match (old, new) {
        (serde_json::Value::Object(old_map), serde_json::Value::Object(new_map)) => {
            // Keys in new (added or changed)
            for (key, new_val) in new_map {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                let old_val = old_map.get(key).unwrap_or(&serde_json::Value::Null);
                let mut sub = compute_diff(old_val, new_val, path);
                changes.append(&mut sub);
            }
            // Keys removed
            for (key, old_val) in old_map {
                if !new_map.contains_key(key) {
                    let path = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{prefix}.{key}")
                    };
                    changes.push(serde_json::json!({
                        "path": path,
                        "old": old_val,
                        "new": null
                    }));
                }
            }
        }
        _ if old != new => {
            // Mask sensitive leaf values in diff output
            let display_old = if is_sensitive_path(&prefix) {
                serde_json::json!("[REDACTED]")
            } else {
                old.clone()
            };
            let display_new = if is_sensitive_path(&prefix) {
                serde_json::json!("[REDACTED]")
            } else {
                new.clone()
            };
            changes.push(serde_json::json!({
                "path": prefix,
                "old": display_old,
                "new": display_new
            }));
        }
        _ => {}
    }
    changes
}

fn is_sensitive_path(path: &str) -> bool {
    matches!(
        path,
        "admin_api.jwt_secret"
            | "admin_api.api_keys"
            | "storage.sql.url"
            | "storage.mongodb.url"
            | "cache.redis.url"
    )
}

fn get_nested<'a>(value: &'a serde_json::Value, path: &[&str]) -> Option<&'a serde_json::Value> {
    let mut cur = value;
    for key in path {
        cur = cur.get(key)?;
    }
    Some(cur)
}

fn get_nested_mut<'a>(
    value: &'a mut serde_json::Value,
    path: &[&str],
) -> Option<&'a mut serde_json::Value> {
    let mut cur = value;
    for key in path {
        cur = cur.get_mut(key)?;
    }
    Some(cur)
}
