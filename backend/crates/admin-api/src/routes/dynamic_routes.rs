use crate::error::ApiError;
use crate::response::ApiResponse;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;
use plugin_host::RouteDefinition;
use serde::{Deserialize, Serialize};

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct RouteListResponse {
    pub total: usize,
    pub routes: Vec<RouteDefinition>,
}

#[derive(Serialize)]
pub struct PluginRouteListResponse {
    pub plugin_id: String,
    pub total: usize,
    pub routes: Vec<RouteDefinition>,
}

// ── Request types ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RegisterRouteRequest {
    /// HTTP method (e.g. "GET", "POST").
    pub method: String,
    /// Route path, e.g. "/api/clans/{id}".
    pub path: String,
    /// Optional human-readable description.
    pub description: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /api/routes — list all routes registered by all plugins.
pub async fn list_routes(State(state): State<AppState>) -> impl IntoResponse {
    let routes = state.route_registry.all_routes();
    let total = routes.len();
    ApiResponse::ok(RouteListResponse { total, routes })
}

/// GET /api/routes/{plugin_id} — list routes for a specific plugin.
pub async fn get_plugin_routes(
    Path(plugin_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let routes = state.route_registry.get_routes(&plugin_id);
    let total = routes.len();
    ApiResponse::ok(PluginRouteListResponse {
        plugin_id,
        total,
        routes,
    })
}

/// POST /api/routes/{plugin_id}/register — register a new route for a plugin.
pub async fn register_route(
    Path(plugin_id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<RegisterRouteRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if req.method.is_empty() {
        return Err(ApiError::bad_request("method must not be empty"));
    }
    if req.path.is_empty() {
        return Err(ApiError::bad_request("path must not be empty"));
    }
    let definition = RouteDefinition {
        method: req.method.to_uppercase(),
        path: req.path.clone(),
        plugin_id: plugin_id.clone(),
        description: req.description,
    };
    state
        .route_registry
        .register(&plugin_id, definition)
        .map_err(|e| ApiError::bad_request(e))?;

    Ok(ApiResponse::<()>::message(format!(
        "route registered for plugin '{plugin_id}'"
    )))
}

/// DELETE /api/routes/{plugin_id} — unregister all routes for a plugin.
pub async fn unregister_plugin_routes(
    Path(plugin_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let count = state.route_registry.unregister_all(&plugin_id);
    ApiResponse::<()>::message(format!(
        "removed {count} route(s) for plugin '{plugin_id}'"
    ))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::build_router;
    use crate::test_support::{admin_get, admin_request, make_state};
    use axum::body::Body;
    use axum::http::StatusCode;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_list_routes_empty() {
        let state = make_state().await;
        let app = build_router(state.clone()).await;

        let resp = app
            .oneshot(
                admin_get(&state, "/api/routes"),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["total"], 0);
    }

    #[tokio::test]
    async fn test_register_and_list_route() {
        let state = make_state().await;

        // Register a route directly via the registry before building the router.
        state
            .route_registry
            .register(
                "io.draox.clans",
                RouteDefinition {
                    method: "GET".to_string(),
                    path: "/api/clans".to_string(),
                    plugin_id: "io.draox.clans".to_string(),
                    description: None,
                },
            )
            .unwrap();

        let app = build_router(state.clone()).await;

        let resp = app
            .oneshot(
                admin_get(&state, "/api/routes/io.draox.clans"),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"]["total"], 1);
    }

    #[tokio::test]
    async fn test_register_route_via_api() {
        let state = make_state().await;
        let app = build_router(state.clone()).await;

        let body = serde_json::json!({
            "method": "POST",
            "path": "/api/messaging/send",
            "description": "Send a message"
        });

        let resp = app
            .oneshot(
                admin_request(&state)
                    .method("POST")
                    .uri("/api/routes/io.draox.messaging/register")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }
}
