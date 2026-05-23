use server_core::{ConnectionId, PluginId, ServerInfo, SessionId};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::broadcast;

// ────────────────────────────────────────────────────────
// Service handle traits — abstract interfaces to server services
// ────────────────────────────────────────────────────────

/// Handle to connection management operations.
pub trait ConnectionHandle: Send + Sync + 'static {
    fn send_to_connection(
        &self,
        connection_id: &ConnectionId,
        data: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = server_core::Result<()>> + Send + '_>>;

    fn send_to_session(
        &self,
        session_id: &SessionId,
        data: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = server_core::Result<()>> + Send + '_>>;

    fn disconnect(
        &self,
        connection_id: &ConnectionId,
        reason: &str,
    ) -> Pin<Box<dyn Future<Output = server_core::Result<()>> + Send + '_>>;

    fn connection_count(&self) -> usize;
}

/// Handle to storage operations (scoped to plugin namespace).
pub trait StorageHandle: Send + Sync + 'static {
    fn get(
        &self,
        key: &str,
    ) -> Pin<Box<dyn Future<Output = server_core::Result<Option<Value>>> + Send + '_>>;

    fn set(
        &self,
        key: &str,
        value: Value,
    ) -> Pin<Box<dyn Future<Output = server_core::Result<()>> + Send + '_>>;

    fn delete(
        &self,
        key: &str,
    ) -> Pin<Box<dyn Future<Output = server_core::Result<bool>> + Send + '_>>;

    fn list_keys(
        &self,
        prefix: &str,
    ) -> Pin<Box<dyn Future<Output = server_core::Result<Vec<String>>> + Send + '_>>;
}

/// Handle to cache operations (scoped to plugin namespace).
pub trait CacheHandle: Send + Sync + 'static {
    fn get(
        &self,
        key: &str,
    ) -> Pin<Box<dyn Future<Output = server_core::Result<Option<Vec<u8>>>> + Send + '_>>;

    fn set(
        &self,
        key: &str,
        value: Vec<u8>,
        ttl_secs: Option<u64>,
    ) -> Pin<Box<dyn Future<Output = server_core::Result<()>> + Send + '_>>;

    fn delete(
        &self,
        key: &str,
    ) -> Pin<Box<dyn Future<Output = server_core::Result<bool>> + Send + '_>>;
}

/// Handle to the event bus for publish/subscribe.
pub trait EventBusHandle: Send + Sync + 'static {
    fn publish(&self, event: server_core::event::ServerEvent);
    fn subscribe(&self, topic: &str) -> broadcast::Receiver<Arc<server_core::event::ServerEvent>>;
}

/// Handle to plugin-scoped logging.
pub trait PluginLoggerHandle: Send + Sync + 'static {
    fn info(&self, msg: &str);
    fn warn(&self, msg: &str);
    fn error(&self, msg: &str);
    fn debug(&self, msg: &str);
}

/// Handle to route registration.
pub trait RouterHandle: Send + Sync + 'static {
    fn register_route(
        &self,
        method: &str,
        path: &str,
        handler_id: &str,
    ) -> server_core::Result<()>;

    fn unregister_route(&self, path: &str) -> server_core::Result<()>;
}

/// Handle to scheduled tasks.
pub trait SchedulerHandle: Send + Sync + 'static {
    fn schedule_once(
        &self,
        delay_secs: u64,
        task_id: &str,
    ) -> server_core::Result<()>;

    fn schedule_interval(
        &self,
        interval_secs: u64,
        task_id: &str,
    ) -> server_core::Result<()>;

    fn cancel(&self, task_id: &str) -> server_core::Result<()>;
}

// ────────────────────────────────────────────────────────
// Plugin context — provided to plugins on activation
// ────────────────────────────────────────────────────────

/// Context provided to a plugin, giving access to server services.
pub struct PluginContext {
    pub plugin_id: PluginId,
    pub server_info: ServerInfo,
    pub config: Value,
    pub connections: Arc<dyn ConnectionHandle>,
    pub storage: Arc<dyn StorageHandle>,
    pub cache: Arc<dyn CacheHandle>,
    pub events: Arc<dyn EventBusHandle>,
    pub logger: Arc<dyn PluginLoggerHandle>,
    pub router: Arc<dyn RouterHandle>,
    pub scheduler: Arc<dyn SchedulerHandle>,
}

impl PluginContext {
    pub fn plugin_id(&self) -> &PluginId {
        &self.plugin_id
    }

    pub fn server_info(&self) -> &ServerInfo {
        &self.server_info
    }

    pub fn config(&self) -> &Value {
        &self.config
    }
}

impl std::fmt::Debug for PluginContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginContext")
            .field("plugin_id", &self.plugin_id)
            .field("server_info", &self.server_info)
            .finish_non_exhaustive()
    }
}
