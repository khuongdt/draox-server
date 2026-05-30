pub mod auth;
pub mod auth_store;
pub mod error;
pub mod response;
pub mod routes;
pub mod seed;
pub mod server;
pub mod state;
pub mod trace_context;

pub use server::{AdminServer, AdminServerConfig};
pub use state::AppState;

#[cfg(test)]
mod tests {
    use super::*;
    use activity_log::metrics::MetricsCollector;
    use activity_log::{ActivityLog, AuditLog};
    use auth::JwtConfig;
    use auth_store::AdminUserStore;
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

    /// Noop handler for constructing TrafficGuard in tests.
    struct TestHandler;

    impl ConnectionHandler for TestHandler {
        fn on_connect<'a>(
            &'a self,
            _info: &'a server_core::ConnectionInfo,
        ) -> BoxFuture<'a, server_core::Result<()>> {
            Box::pin(async { Ok(()) })
        }

        fn on_data<'a>(
            &'a self,
            _conn_id: &'a ConnectionId,
            _data: &'a [u8],
        ) -> BoxFuture<'a, ()> {
            Box::pin(async {})
        }

        fn on_disconnect<'a>(
            &'a self,
            _conn_id: &'a ConnectionId,
            _reason: &'a str,
        ) -> BoxFuture<'a, ()> {
            Box::pin(async {})
        }

        fn on_error<'a>(
            &'a self,
            _conn_id: &'a ConnectionId,
            _error: &'a Error,
        ) -> BoxFuture<'a, ()> {
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
            marketplace: Arc::new(FullMarketplaceRegistry::new()),
            route_registry: Arc::new(RouteRegistry::new()),
            cache,
            storage,
            jwt_config: JwtConfig::default(),
            auth_store,
            config: Arc::new(std::sync::RwLock::new(config)),
            config_path: String::new(),
        }
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let state = make_state().await;
        let app = routes::build_router(state).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["status"], "ok");
    }

    #[tokio::test]
    async fn test_info_endpoint() {
        let state = make_state().await;
        let app = routes::build_router(state).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/info")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"]["name"], "Draox Server");
    }

    #[tokio::test]
    async fn test_connections_endpoint() {
        let state = make_state().await;
        let app = routes::build_router(state).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/connections")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"]["total"], 0);
    }

    #[tokio::test]
    async fn test_sessions_endpoint() {
        let state = make_state().await;
        let app = routes::build_router(state).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/sessions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"]["total"], 0);
    }

    #[tokio::test]
    async fn test_plugins_endpoint() {
        let state = make_state().await;
        let app = routes::build_router(state).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/plugins")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"]["total"], 0);
    }

    #[tokio::test]
    async fn test_guard_stats_endpoint() {
        let state = make_state().await;
        let app = routes::build_router(state).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/guard/stats")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"]["active_bans"], 0);
    }

    #[tokio::test]
    async fn test_metrics_endpoint() {
        let state = make_state().await;

        // Record some metrics
        state.metrics.increment_connections();
        state.metrics.record_bytes_received(1024);
        state.metrics.increment_requests();

        let app = routes::build_router(state).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"]["connections_active"], 1);
        assert_eq!(json["data"]["bytes_received"], 1024); // renamed via #[serde(rename)]
        assert_eq!(json["data"]["requests_total"], 1);
    }

    #[tokio::test]
    async fn test_connection_not_found() {
        let state = make_state().await;
        let app = routes::build_router(state).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/connections/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_ban_and_unban() {
        let state = make_state().await;

        // Ban
        let app = routes::build_router(state.clone()).await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/guard/ban")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"ip":"10.0.0.1"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Verify ban is active
        assert_eq!(state.traffic_guard.ban_manager().active_ban_count(), 1);

        // Unban
        let app = routes::build_router(state.clone()).await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/guard/unban")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"ip":"10.0.0.1"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        assert_eq!(state.traffic_guard.ban_manager().active_ban_count(), 0);
    }
}
