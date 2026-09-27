//! Shared fixtures for router-level tests: an in-memory `AppState`, seeded admin users,
//! and helpers that attach credentials now that every built-in route requires auth.

use crate::auth::{create_jwt_token, AdminRole, JwtConfig};
use crate::auth_store::{AdminUser, AdminUserStore};
use crate::state::AppState;
use activity_log::metrics::MetricsCollector;
use activity_log::{ActivityLog, AuditLog};
use axum::body::Body;
use axum::http::{request::Builder, Request};
use billing::UsageTracker;
use connection_manager::SessionManager;
use plugin_host::{ContextBuilder, FullMarketplaceRegistry, PluginRegistry, RouteRegistry};
use server_config::model::{SessionConfig, TrafficGuardConfig};
use server_core::event::EventBus;
use server_core::{ConnectionId, Error, ServerInfo};
use socket_server::handler::{BoxFuture, ConnectionHandler};
use socket_server::tracker::ConnectionTracker;
use std::sync::Arc;
use traffic_guard::TrafficGuard;

/// Noop handler for constructing `TrafficGuard` in tests.
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

/// In-memory state with one seeded user per role (`admin`, `operator`, `viewer`).
///
/// Password hashes are placeholders: tests authenticate with minted JWTs, not passwords.
pub async fn make_state() -> AppState {
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
    let cache: Arc<dyn cache_layer::CacheBackend> = Arc::new(cache_layer::MemoryCache::new(
        &server_config::model::MemoryCacheConfig::default(),
    ));
    let storage: Arc<dyn data_store::StorageBackend> =
        Arc::new(data_store::SqliteStorage::new_in_memory().await.unwrap());
    let auth_store = Arc::new(AdminUserStore::new(Arc::clone(&storage)));
    for (username, role) in [
        ("admin", AdminRole::Admin),
        ("operator", AdminRole::Operator),
        ("viewer", AdminRole::Viewer),
    ] {
        auth_store
            .set(&AdminUser {
                username: username.to_string(),
                password_hash: String::new(),
                role,
                banned: false,
            })
            .await
            .unwrap();
    }
    let ctx_builder = ContextBuilder::new(
        ServerInfo::default(),
        Arc::clone(&event_bus),
        Arc::clone(&cache),
    );
    let plugin_registry = Arc::new(PluginRegistry::new(ctx_builder, Arc::clone(&event_bus)));

    let mut config = server_config::DraoxConfig::default();
    // Generous budget so ordinary tests never trip the limiter.
    config.admin_api.rate_limit_per_sec = 10_000;
    config.admin_api.login_rate_limit_per_min = 10_000;

    AppState {
        connection_tracker: tracker,
        session_manager: session_mgr,
        traffic_guard: guard,
        plugin_registry,
        activity_log: Arc::new(ActivityLog::new(10000)),
        metrics: Arc::new(MetricsCollector::new()),
        usage_tracker: Arc::new(UsageTracker::new()),
        audit_log: Arc::new(AuditLog::new(10000)),
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

/// A valid bearer token for `username` with `role`, signed with the state's secret.
pub fn token(state: &AppState, username: &str, role: AdminRole) -> String {
    create_jwt_token(username, role, &state.jwt_config).unwrap()
}

/// Request builder pre-authenticated as the seeded `admin` user.
pub fn admin_request(state: &AppState) -> Builder {
    as_user(state, "admin", AdminRole::Admin)
}

/// Request builder authenticated as `username` with `role`.
pub fn as_user(state: &AppState, username: &str, role: AdminRole) -> Builder {
    Request::builder().header(
        "authorization",
        format!("Bearer {}", token(state, username, role)),
    )
}

/// `GET uri` as the seeded admin.
pub fn admin_get(state: &AppState, uri: &str) -> Request<Body> {
    admin_request(state).uri(uri).body(Body::empty()).unwrap()
}
