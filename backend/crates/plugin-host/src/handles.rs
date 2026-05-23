use cache_layer::CacheBackend;
use dashmap::DashMap;
use plugin_sdk::context::{
    CacheHandle, ConnectionHandle, EventBusHandle, PluginLoggerHandle, RouterHandle,
    SchedulerHandle, StorageHandle,
};
use server_core::event::{EventBus, ServerEvent};
use server_core::{ConnectionId, Error, PluginId, SessionId};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::broadcast;

// ────────────────────────────────────────────────────────
// NoopConnectionHandle — returns errors (no real connection layer)
// ────────────────────────────────────────────────────────

pub struct NoopConnectionHandle;

impl ConnectionHandle for NoopConnectionHandle {
    fn send_to_connection(
        &self,
        connection_id: &ConnectionId,
        _data: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = server_core::Result<()>> + Send + '_>> {
        let id = connection_id.to_string();
        Box::pin(async move {
            Err(Error::Connection(format!(
                "no connection layer available for {id}"
            )))
        })
    }

    fn send_to_session(
        &self,
        session_id: &SessionId,
        _data: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = server_core::Result<()>> + Send + '_>> {
        let id = session_id.to_string();
        Box::pin(async move {
            Err(Error::Connection(format!(
                "no connection layer available for session {id}"
            )))
        })
    }

    fn disconnect(
        &self,
        connection_id: &ConnectionId,
        _reason: &str,
    ) -> Pin<Box<dyn Future<Output = server_core::Result<()>> + Send + '_>> {
        let id = connection_id.to_string();
        Box::pin(async move {
            Err(Error::Connection(format!(
                "no connection layer available for {id}"
            )))
        })
    }

    fn connection_count(&self) -> usize {
        0
    }
}

// ────────────────────────────────────────────────────────
// InMemoryStorageHandle — DashMap-based namespace-scoped storage
// ────────────────────────────────────────────────────────

pub struct InMemoryStorageHandle {
    namespace: String,
    data: Arc<DashMap<String, Value>>,
}

impl InMemoryStorageHandle {
    pub fn new(namespace: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            data: Arc::new(DashMap::new()),
        }
    }

    fn scoped_key(&self, key: &str) -> String {
        format!("{}:{}", self.namespace, key)
    }
}

impl StorageHandle for InMemoryStorageHandle {
    fn get(
        &self,
        key: &str,
    ) -> Pin<Box<dyn Future<Output = server_core::Result<Option<Value>>> + Send + '_>> {
        let scoped = self.scoped_key(key);
        let data = Arc::clone(&self.data);
        Box::pin(async move { Ok(data.get(&scoped).map(|r| r.value().clone())) })
    }

    fn set(
        &self,
        key: &str,
        value: Value,
    ) -> Pin<Box<dyn Future<Output = server_core::Result<()>> + Send + '_>> {
        let scoped = self.scoped_key(key);
        let data = Arc::clone(&self.data);
        Box::pin(async move {
            data.insert(scoped, value);
            Ok(())
        })
    }

    fn delete(
        &self,
        key: &str,
    ) -> Pin<Box<dyn Future<Output = server_core::Result<bool>> + Send + '_>> {
        let scoped = self.scoped_key(key);
        let data = Arc::clone(&self.data);
        Box::pin(async move { Ok(data.remove(&scoped).is_some()) })
    }

    fn list_keys(
        &self,
        prefix: &str,
    ) -> Pin<Box<dyn Future<Output = server_core::Result<Vec<String>>> + Send + '_>> {
        let full_prefix = self.scoped_key(prefix);
        let ns_prefix = format!("{}:", self.namespace);
        let data = Arc::clone(&self.data);
        Box::pin(async move {
            let keys: Vec<String> = data
                .iter()
                .filter(|e| e.key().starts_with(&full_prefix))
                .map(|e| e.key().strip_prefix(&ns_prefix).unwrap_or(e.key()).to_string())
                .collect();
            Ok(keys)
        })
    }
}

// ────────────────────────────────────────────────────────
// BackendCacheHandle — delegates to Arc<dyn CacheBackend>
// ────────────────────────────────────────────────────────

/// Plugin-scoped cache handle backed by the server's [`CacheBackend`].
///
/// All keys are automatically prefixed with `plugin:{namespace}:` to prevent
/// collisions between plugins and server-internal cache entries.
pub struct BackendCacheHandle {
    namespace: String,
    backend: Arc<dyn CacheBackend>,
}

impl BackendCacheHandle {
    pub fn new(namespace: impl Into<String>, backend: Arc<dyn CacheBackend>) -> Self {
        Self {
            namespace: namespace.into(),
            backend,
        }
    }

    fn scoped_key(&self, key: &str) -> String {
        format!("plugin:{}:{}", self.namespace, key)
    }
}

impl CacheHandle for BackendCacheHandle {
    fn get(
        &self,
        key: &str,
    ) -> Pin<Box<dyn Future<Output = server_core::Result<Option<Vec<u8>>>> + Send + '_>> {
        let scoped = self.scoped_key(key);
        let backend = Arc::clone(&self.backend);
        Box::pin(async move { backend.get(&scoped).await })
    }

    fn set(
        &self,
        key: &str,
        value: Vec<u8>,
        ttl_secs: Option<u64>,
    ) -> Pin<Box<dyn Future<Output = server_core::Result<()>> + Send + '_>> {
        let scoped = self.scoped_key(key);
        let backend = Arc::clone(&self.backend);
        Box::pin(async move { backend.set(&scoped, value, ttl_secs).await })
    }

    fn delete(
        &self,
        key: &str,
    ) -> Pin<Box<dyn Future<Output = server_core::Result<bool>> + Send + '_>> {
        let scoped = self.scoped_key(key);
        let backend = Arc::clone(&self.backend);
        Box::pin(async move { backend.delete(&scoped).await })
    }
}

// ────────────────────────────────────────────────────────
// EventBusHandleImpl — wraps Arc<EventBus>
// ────────────────────────────────────────────────────────

pub struct EventBusHandleImpl {
    bus: Arc<EventBus>,
}

impl EventBusHandleImpl {
    pub fn new(bus: Arc<EventBus>) -> Self {
        Self { bus }
    }
}

impl EventBusHandle for EventBusHandleImpl {
    fn publish(&self, event: ServerEvent) {
        self.bus.publish(event);
    }

    fn subscribe(&self, topic: &str) -> broadcast::Receiver<Arc<ServerEvent>> {
        self.bus.subscribe_topic(topic)
    }
}

// ────────────────────────────────────────────────────────
// PluginLoggerImpl — uses tracing with plugin_id context
// ────────────────────────────────────────────────────────

pub struct PluginLoggerImpl {
    plugin_id: PluginId,
}

impl PluginLoggerImpl {
    pub fn new(plugin_id: PluginId) -> Self {
        Self { plugin_id }
    }
}

impl PluginLoggerHandle for PluginLoggerImpl {
    fn info(&self, msg: &str) {
        tracing::info!(plugin_id = %self.plugin_id, "{}", msg);
    }

    fn warn(&self, msg: &str) {
        tracing::warn!(plugin_id = %self.plugin_id, "{}", msg);
    }

    fn error(&self, msg: &str) {
        tracing::error!(plugin_id = %self.plugin_id, "{}", msg);
    }

    fn debug(&self, msg: &str) {
        tracing::debug!(plugin_id = %self.plugin_id, "{}", msg);
    }
}

// ────────────────────────────────────────────────────────
// NoopRouterHandle — stub (admin-api will provide real impl)
// ────────────────────────────────────────────────────────

pub struct NoopRouterHandle;

impl RouterHandle for NoopRouterHandle {
    fn register_route(
        &self,
        _method: &str,
        _path: &str,
        _handler_id: &str,
    ) -> server_core::Result<()> {
        Ok(())
    }

    fn unregister_route(&self, _path: &str) -> server_core::Result<()> {
        Ok(())
    }
}

// ────────────────────────────────────────────────────────
// NoopSchedulerHandle — stub (future implementation)
// ────────────────────────────────────────────────────────

pub struct NoopSchedulerHandle;

impl SchedulerHandle for NoopSchedulerHandle {
    fn schedule_once(&self, _delay_secs: u64, _task_id: &str) -> server_core::Result<()> {
        Ok(())
    }

    fn schedule_interval(&self, _interval_secs: u64, _task_id: &str) -> server_core::Result<()> {
        Ok(())
    }

    fn cancel(&self, _task_id: &str) -> server_core::Result<()> {
        Ok(())
    }
}

// ────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_storage_handle() {
        let storage = InMemoryStorageHandle::new("test.plugin");

        // Set and get
        storage
            .set("key1", serde_json::json!({"name": "test"}))
            .await
            .unwrap();
        let val = storage.get("key1").await.unwrap();
        assert_eq!(val, Some(serde_json::json!({"name": "test"})));

        // Delete
        let deleted = storage.delete("key1").await.unwrap();
        assert!(deleted);
        let val = storage.get("key1").await.unwrap();
        assert!(val.is_none());

        // Delete non-existent
        let deleted = storage.delete("nope").await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_in_memory_storage_list_keys() {
        let storage = InMemoryStorageHandle::new("ns");
        storage
            .set("users:alice", serde_json::json!(1))
            .await
            .unwrap();
        storage
            .set("users:bob", serde_json::json!(2))
            .await
            .unwrap();
        storage
            .set("settings:theme", serde_json::json!("dark"))
            .await
            .unwrap();

        let keys = storage.list_keys("users:").await.unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"users:alice".to_string()));
        assert!(keys.contains(&"users:bob".to_string()));
    }

    #[tokio::test]
    async fn test_backend_cache_handle() {
        use cache_layer::MemoryCache;
        use server_config::model::MemoryCacheConfig;

        let backend: Arc<dyn CacheBackend> = Arc::new(MemoryCache::new(&MemoryCacheConfig {
            max_capacity: 100,
            ttl_secs: 300,
        }));
        let cache = BackendCacheHandle::new("test", backend);

        cache.set("k1", vec![1, 2, 3], None).await.unwrap();
        let val = cache.get("k1").await.unwrap();
        assert_eq!(val, Some(vec![1, 2, 3]));

        let deleted = cache.delete("k1").await.unwrap();
        assert!(deleted);
        assert!(cache.get("k1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_event_bus_handle() {
        let bus = Arc::new(EventBus::new(16));
        let handle = EventBusHandleImpl::new(Arc::clone(&bus));

        let mut rx = handle.subscribe("plugin");
        handle.publish(ServerEvent::PluginActivated {
            plugin_id: PluginId::from_str("io.draox.test"),
        });

        let event = rx.recv().await.unwrap();
        assert!(matches!(&*event, ServerEvent::PluginActivated { .. }));
    }

    #[test]
    fn test_noop_connection_handle_count() {
        let handle = NoopConnectionHandle;
        assert_eq!(handle.connection_count(), 0);
    }

    #[test]
    fn test_noop_router_handle() {
        let handle = NoopRouterHandle;
        assert!(handle
            .register_route("GET", "/test", "handler1")
            .is_ok());
        assert!(handle.unregister_route("/test").is_ok());
    }

    #[test]
    fn test_noop_scheduler_handle() {
        let handle = NoopSchedulerHandle;
        assert!(handle.schedule_once(60, "task1").is_ok());
        assert!(handle.schedule_interval(30, "task2").is_ok());
        assert!(handle.cancel("task1").is_ok());
    }
}
