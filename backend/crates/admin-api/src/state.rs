use crate::auth::JwtConfig;
use crate::auth_store::AdminUserStore;
use activity_log::metrics::MetricsCollector;
use activity_log::{ActivityLog, AuditLog};
use billing::UsageTracker;
use cache_layer::CacheBackend;
use connection_manager::SessionManager;
use data_store::StorageBackend;
use plugin_host::{FullMarketplaceRegistry, PluginRegistry, RouteRegistry};
use server_core::event::EventBus;
use socket_server::tracker::ConnectionTracker;
use std::sync::Arc;
use traffic_guard::TrafficGuard;

/// Shared application state for all admin-api routes.
#[derive(Clone)]
pub struct AppState {
    pub connection_tracker: Arc<ConnectionTracker>,
    pub session_manager: Arc<SessionManager>,
    pub traffic_guard: Arc<TrafficGuard>,
    pub plugin_registry: Arc<PluginRegistry>,
    pub activity_log: Arc<ActivityLog>,
    pub metrics: Arc<MetricsCollector>,
    pub usage_tracker: Arc<UsageTracker>,
    pub event_bus: Arc<EventBus>,
    pub audit_log: Arc<AuditLog>,
    pub marketplace: Arc<FullMarketplaceRegistry>,
    pub route_registry: Arc<RouteRegistry>,
    pub cache: Arc<dyn CacheBackend>,
    pub storage: Arc<dyn StorageBackend>,
    pub jwt_config: JwtConfig,
    pub auth_store: Arc<AdminUserStore>,
}
