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
    use crate::auth::JwtConfig;
    use crate::auth_store::AdminUserStore;
    use crate::routes::build_router;
    use crate::state::AppState;
    use activity_log::metrics::MetricsCollector;
    use activity_log::{ActivityLog, AuditLog};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use billing::UsageTracker;
    use connection_manager::SessionManager;
    use plugin_host::{ContextBuilder, FullMarketplaceRegistry, PluginRegistry, RouteRegistry};
    use server_config::model::{SessionConfig, TrafficGuardConfig};
    use server_core::event::EventBus;
    use server_core::{ConnectionId, Error, ServerInfo};
    use socket_server::handler::{BoxFuture, ConnectionHandler};
    use socket_server::tracker::ConnectionTracker;
    use std::sync::Arc;
    use tower::ServiceExt;
    use traffic_guard::TrafficGuard;

    struct TestHandler;
    impl ConnectionHandler for TestHandler {
        fn on_connect<'a>(
            &'a self,
            _info: &'a server_core::ConnectionInfo,
        ) -> BoxFuture<'a, server_core::Result<()>> {
            Box::pin(async { Ok(()) })
        }
        fn on_data<'a>(&'a self, _: &'a ConnectionId, _: &'a [u8]) -> BoxFuture<'a, ()> {
            Box::pin(async {})
        }
        fn on_disconnect<'a>(&'a self, _: &'a ConnectionId, _: &'a str) -> BoxFuture<'a, ()> {
            Box::pin(async {})
        }
        fn on_error<'a>(&'a self, _: &'a ConnectionId, _: &'a Error) -> BoxFuture<'a, ()> {
            Box::pin(async {})
        }
    }

    async fn make_state() -> AppState {
        let event_bus = Arc::new(EventBus::new(16));
        let tracker = Arc::new(ConnectionTracker::new(1000, 100));
        let session_mgr = Arc::new(SessionManager::new(
            SessionConfig::default(),
            Arc::clone(&event_bus),
        ));
        let guard = Arc::new(TrafficGuard::new(
            TrafficGuardConfig::default(),
            Arc::new(TestHandler),
            Arc::clone(&event_bus),
        ));
        let cache: Arc<dyn cache_layer::CacheBackend> = Arc::new(
            cache_layer::MemoryCache::new(&server_config::model::MemoryCacheConfig::default()),
        );
        let storage: Arc<dyn data_store::StorageBackend> = Arc::new(
            data_store::SqliteStorage::new_in_memory().await.unwrap(),
        );
        let auth_store = Arc::new(AdminUserStore::new(Arc::clone(&storage)));
        let ctx_builder = ContextBuilder::new(ServerInfo::default(), Arc::clone(&event_bus), Arc::clone(&cache));
        let plugin_registry = Arc::new(PluginRegistry::new(ctx_builder, Arc::clone(&event_bus)));
        let activity_log = Arc::new(ActivityLog::new(10000));
        let audit_log = Arc::new(AuditLog::new(10000));
        let metrics = Arc::new(MetricsCollector::new());
        let usage_tracker = Arc::new(UsageTracker::new());
        let marketplace = Arc::new(FullMarketplaceRegistry::new());
        let route_registry = Arc::new(RouteRegistry::new());
        let config = server_config::DraoxConfig::default();
        AppState {
            connection_tracker: tracker,
            session_manager: session_mgr,
            traffic_guard: guard,
            plugin_registry,
            activity_log,
            metrics,
            usage_tracker,
            audit_log,
            event_bus,
            marketplace,
            route_registry,
            cache,
            storage,
            jwt_config: JwtConfig::default(),
            auth_store,
            config: Arc::new(std::sync::RwLock::new(config)),
            config_path: String::new(),
        }
    }

    #[tokio::test]
    async fn test_list_routes_empty() {
        let state = make_state().await;
        let app = build_router(state).await;

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/routes")
                    .body(Body::empty())
                    .unwrap(),
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

        let app = build_router(state).await;

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/routes/io.draox.clans")
                    .body(Body::empty())
                    .unwrap(),
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
        let app = build_router(state).await;

        let body = serde_json::json!({
            "method": "POST",
            "path": "/api/messaging/send",
            "description": "Send a message"
        });

        let resp = app
            .oneshot(
                Request::builder()
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
