use crate::response::ApiResponse;
use crate::state::AppState;
use axum::extract::State;
use axum::response::IntoResponse;
use chrono::Utc;
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
    pub connections: usize,
    pub sessions: usize,
    pub plugins: usize,
}

#[derive(Serialize)]
pub struct ServerInfoResponse {
    pub name: String,
    pub version: String,
    pub uptime_secs: i64,
    pub connections: usize,
    pub sessions: usize,
    pub plugins: usize,
}

/// GET /api/health
pub async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let resp = HealthResponse {
        status: "ok".to_string(),
        timestamp: Utc::now().to_rfc3339(),
        connections: state.connection_tracker.count(),
        sessions: state.session_manager.session_count(),
        plugins: state.plugin_registry.count(),
    };
    ApiResponse::ok(resp)
}

/// GET /api/info
pub async fn info(State(state): State<AppState>) -> impl IntoResponse {
    let resp = ServerInfoResponse {
        name: "Draox Server".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: 0,
        connections: state.connection_tracker.count(),
        sessions: state.session_manager.session_count(),
        plugins: state.plugin_registry.count(),
    };
    ApiResponse::ok(resp)
}

#[derive(Serialize)]
pub struct AggregateHealthResponse {
    pub overall: String,
    pub components: Vec<ComponentHealth>,
}

#[derive(Serialize)]
pub struct ComponentHealth {
    pub name: String,
    pub status: String,
    pub details: Option<String>,
}

/// GET /api/health/detailed — aggregate health across all components.
pub async fn health_detailed(State(state): State<AppState>) -> impl IntoResponse {
    let mut components = Vec::new();
    let overall_healthy = true;

    // Connections
    let conn_count = state.connection_tracker.count();
    components.push(ComponentHealth {
        name: "connections".to_string(),
        status: "ok".to_string(),
        details: Some(format!("{conn_count} active")),
    });

    // Sessions
    let session_count = state.session_manager.session_count();
    components.push(ComponentHealth {
        name: "sessions".to_string(),
        status: "ok".to_string(),
        details: Some(format!("{session_count} active")),
    });

    // Traffic guard
    let ban_count = state.traffic_guard.ban_manager().active_ban_count();
    components.push(ComponentHealth {
        name: "traffic_guard".to_string(),
        status: "ok".to_string(),
        details: Some(format!("{ban_count} active bans")),
    });

    // Plugins
    let plugin_count = state.plugin_registry.count();
    components.push(ComponentHealth {
        name: "plugins".to_string(),
        status: "ok".to_string(),
        details: Some(format!("{plugin_count} registered")),
    });

    // Metrics/errors check
    let snapshot = state.metrics.snapshot();
    if snapshot.errors_total > 0 {
        components.push(ComponentHealth {
            name: "error_rate".to_string(),
            status: "warning".to_string(),
            details: Some(format!("{} total errors", snapshot.errors_total)),
        });
        // Don't set overall_healthy = false for warnings
    } else {
        components.push(ComponentHealth {
            name: "error_rate".to_string(),
            status: "ok".to_string(),
            details: None,
        });
    }

    let overall = if overall_healthy {
        "healthy".to_string()
    } else {
        "degraded".to_string()
    };

    ApiResponse::ok(AggregateHealthResponse {
        overall,
        components,
    })
}
