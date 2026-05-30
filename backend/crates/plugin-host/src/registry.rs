use crate::context_builder::ContextBuilder;
use crate::lifecycle::validate_transition;
use axum::Router;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use plugin_sdk::traits::{Plugin, PluginHealth, PluginState, WsActionContext};
use server_core::event::{EventBus, ServerEvent};
use server_core::{Error, PluginId, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// Policy controlling how many times and how often a plugin may be restarted.
#[derive(Debug, Clone)]
pub struct RestartPolicy {
    /// Maximum restart attempts allowed within the cooldown window.
    pub max_attempts: u32,
    /// Rolling window / base cooldown between restarts.
    pub cooldown: Duration,
    /// Multiplier applied to the cooldown after each successive restart.
    pub backoff_multiplier: f64,
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            cooldown: Duration::from_secs(5),
            backoff_multiplier: 2.0,
        }
    }
}

/// Tracks restart history for a single plugin.
struct RestartHistory {
    /// Timestamps of recent restarts (kept for the lifetime of the registry).
    attempts: Vec<Instant>,
    /// Total restarts ever recorded for this plugin.
    total_restarts: u32,
}

/// Internal entry stored per plugin.
struct PluginEntry {
    plugin: Mutex<Box<dyn Plugin>>,
    state: PluginState,
    registered_at: DateTime<Utc>,
    activated_at: Option<DateTime<Utc>>,
}

/// Summary info about a plugin (safe to serialize).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: PluginId,
    pub name: String,
    pub version: String,
    pub state: PluginState,
    pub registered_at: DateTime<Utc>,
    pub activated_at: Option<DateTime<Utc>>,
}

/// Plugin registry — manages registration, lifecycle, and state.
pub struct PluginRegistry {
    plugins: DashMap<PluginId, PluginEntry>,
    context_builder: ContextBuilder,
    event_bus: Arc<EventBus>,
    restart_tracker: DashMap<PluginId, RestartHistory>,
}

impl PluginRegistry {
    pub fn new(context_builder: ContextBuilder, event_bus: Arc<EventBus>) -> Self {
        Self {
            plugins: DashMap::new(),
            context_builder,
            event_bus,
            restart_tracker: DashMap::new(),
        }
    }

    /// Register a built-in plugin. Starts in `Installed` state.
    pub fn register_builtin(&self, plugin: Box<dyn Plugin>) -> Result<()> {
        let id = plugin.id().clone();
        if self.plugins.contains_key(&id) {
            return Err(Error::Plugin {
                plugin_id: id.to_string(),
                message: "plugin already registered".to_string(),
            });
        }

        info!(plugin_id = %id, name = plugin.name(), version = plugin.version(), "plugin registered");

        self.plugins.insert(
            id,
            PluginEntry {
                plugin: Mutex::new(plugin),
                state: PluginState::Installed,
                registered_at: Utc::now(),
                activated_at: None,
            },
        );
        Ok(())
    }

    /// Activate a plugin: Installed → ActiveEnabled.
    pub async fn activate(&self, id: &PluginId) -> Result<()> {
        // Validate transition
        let current_state = self.get_state(id)?;
        validate_transition(current_state, PluginState::ActiveEnabled)?;

        // Build context and call plugin.activate()
        let config = serde_json::Value::Object(serde_json::Map::new());
        let ctx = self.context_builder.build(id, config);

        let entry = self
            .plugins
            .get(id)
            .ok_or_else(|| Error::PluginNotFound(id.to_string()))?;

        let mut plugin = entry.plugin.lock().await;
        plugin.activate(ctx).await.map_err(|e| Error::PluginActivation {
            plugin_id: id.to_string(),
            reason: e.to_string(),
        })?;
        drop(plugin);
        drop(entry);

        // Update state
        if let Some(mut entry) = self.plugins.get_mut(id) {
            entry.state = PluginState::ActiveEnabled;
            entry.activated_at = Some(Utc::now());
        }

        self.event_bus
            .publish(ServerEvent::PluginActivated { plugin_id: id.clone() });
        info!(plugin_id = %id, "plugin activated");
        Ok(())
    }

    /// Deactivate a plugin: Active* → Installed.
    pub async fn deactivate(&self, id: &PluginId) -> Result<()> {
        let current_state = self.get_state(id)?;
        validate_transition(current_state, PluginState::Installed)?;

        let entry = self
            .plugins
            .get(id)
            .ok_or_else(|| Error::PluginNotFound(id.to_string()))?;

        let mut plugin = entry.plugin.lock().await;
        if let Err(e) = plugin.deactivate().await {
            error!(plugin_id = %id, error = %e, "plugin deactivation error (continuing)");
        }
        drop(plugin);
        drop(entry);

        if let Some(mut entry) = self.plugins.get_mut(id) {
            entry.state = PluginState::Installed;
            entry.activated_at = None;
        }

        self.event_bus
            .publish(ServerEvent::PluginDeactivated { plugin_id: id.clone() });
        info!(plugin_id = %id, "plugin deactivated");
        Ok(())
    }

    /// Enable a plugin: ActiveDisabled → ActiveEnabled.
    pub async fn enable(&self, id: &PluginId) -> Result<()> {
        let current_state = self.get_state(id)?;
        if current_state != PluginState::ActiveDisabled {
            return Err(Error::Plugin {
                plugin_id: id.to_string(),
                message: format!("enable requires ActiveDisabled state, got {current_state:?}"),
            });
        }
        validate_transition(current_state, PluginState::ActiveEnabled)?;

        let entry = self
            .plugins
            .get(id)
            .ok_or_else(|| Error::PluginNotFound(id.to_string()))?;

        let mut plugin = entry.plugin.lock().await;
        plugin.on_enable().await?;
        drop(plugin);
        drop(entry);

        if let Some(mut entry) = self.plugins.get_mut(id) {
            entry.state = PluginState::ActiveEnabled;
        }

        self.event_bus
            .publish(ServerEvent::PluginEnabled { plugin_id: id.clone() });
        debug!(plugin_id = %id, "plugin enabled");
        Ok(())
    }

    /// Disable a plugin: ActiveEnabled → ActiveDisabled.
    pub async fn disable(&self, id: &PluginId) -> Result<()> {
        let current_state = self.get_state(id)?;
        if current_state != PluginState::ActiveEnabled {
            return Err(Error::Plugin {
                plugin_id: id.to_string(),
                message: format!("disable requires ActiveEnabled state, got {current_state:?}"),
            });
        }
        validate_transition(current_state, PluginState::ActiveDisabled)?;

        let entry = self
            .plugins
            .get(id)
            .ok_or_else(|| Error::PluginNotFound(id.to_string()))?;

        let mut plugin = entry.plugin.lock().await;
        plugin.on_disable().await?;
        drop(plugin);
        drop(entry);

        if let Some(mut entry) = self.plugins.get_mut(id) {
            entry.state = PluginState::ActiveDisabled;
        }

        self.event_bus
            .publish(ServerEvent::PluginDisabled { plugin_id: id.clone() });
        debug!(plugin_id = %id, "plugin disabled");
        Ok(())
    }

    /// Run health check on a plugin.
    pub async fn health_check(&self, id: &PluginId) -> Result<PluginHealth> {
        let entry = self
            .plugins
            .get(id)
            .ok_or_else(|| Error::PluginNotFound(id.to_string()))?;

        let plugin = entry.plugin.lock().await;
        Ok(plugin.health_check().await)
    }

    /// Get the current state of a plugin.
    pub fn get_state(&self, id: &PluginId) -> Result<PluginState> {
        self.plugins
            .get(id)
            .map(|e| e.state)
            .ok_or_else(|| Error::PluginNotFound(id.to_string()))
    }

    /// List all registered plugins.
    pub fn list(&self) -> Vec<PluginInfo> {
        self.plugins
            .iter()
            .map(|entry| {
                // We can't call async methods here, so we read cached info.
                // The plugin name/version are accessible via the trait, but
                // we'd need to lock. Instead, we store id from the key.
                PluginInfo {
                    id: entry.key().clone(),
                    name: String::new(), // filled by caller if needed
                    version: String::new(),
                    state: entry.state,
                    registered_at: entry.registered_at,
                    activated_at: entry.activated_at,
                }
            })
            .collect()
    }

    /// Get info for a specific plugin (with name/version from the plugin trait).
    pub async fn get_info(&self, id: &PluginId) -> Result<PluginInfo> {
        let entry = self
            .plugins
            .get(id)
            .ok_or_else(|| Error::PluginNotFound(id.to_string()))?;

        let plugin = entry.plugin.lock().await;
        Ok(PluginInfo {
            id: id.clone(),
            name: plugin.name().to_string(),
            version: plugin.version().to_string(),
            state: entry.state,
            registered_at: entry.registered_at,
            activated_at: entry.activated_at,
        })
    }

    /// Restart a plugin: deactivate -> activate. Returns error if plugin is not active.
    pub async fn restart(&self, id: &PluginId) -> Result<()> {
        let state = self.get_state(id)?;
        if !state.is_active() {
            return Err(Error::Plugin {
                plugin_id: id.to_string(),
                message: "cannot restart: plugin is not active".to_string(),
            });
        }
        self.deactivate(id).await?;
        self.activate(id).await?;
        Ok(())
    }

    /// Restart a plugin with policy enforcement.
    ///
    /// Returns `Ok(true)` if the restart was performed, `Ok(false)` if the
    /// policy blocked the restart (too many recent attempts or backoff not yet
    /// elapsed), or `Err` if the underlying restart fails.
    pub async fn restart_with_policy(
        &self,
        id: &PluginId,
        policy: &RestartPolicy,
    ) -> Result<bool> {
        let now = Instant::now();

        // Prune attempts older than the cooldown window and compute backoff.
        let (attempts_in_window, last_attempt, total) = {
            let mut entry = self.restart_tracker.entry(id.clone()).or_insert_with(|| {
                RestartHistory {
                    attempts: Vec::new(),
                    total_restarts: 0,
                }
            });

            // Prune stale entries outside the cooldown window.
            entry
                .attempts
                .retain(|t| now.duration_since(*t) < policy.cooldown);

            let attempts_in_window = entry.attempts.len() as u32;
            let last_attempt = entry.attempts.last().copied();
            let total = entry.total_restarts;
            (attempts_in_window, last_attempt, total)
        };

        // Check max attempts within the rolling window.
        if attempts_in_window >= policy.max_attempts {
            warn!(
                plugin_id = %id,
                attempts = attempts_in_window,
                max = policy.max_attempts,
                "restart blocked: max attempts reached within cooldown window"
            );
            return Ok(false);
        }

        // Enforce exponential backoff: minimum wait = cooldown * multiplier^(attempt index).
        if let Some(last) = last_attempt {
            let elapsed = now.duration_since(last);
            // Backoff index is the current number of attempts already recorded.
            let backoff_secs = policy.cooldown.as_secs_f64()
                * policy.backoff_multiplier.powi(attempts_in_window as i32);
            let required = Duration::from_secs_f64(backoff_secs);
            if elapsed < required {
                warn!(
                    plugin_id = %id,
                    elapsed_ms = elapsed.as_millis(),
                    required_ms = required.as_millis(),
                    "restart blocked: backoff period not yet elapsed"
                );
                return Ok(false);
            }
        }

        // Perform the actual restart.
        self.restart(id).await?;

        // Record this restart attempt.
        {
            let mut entry = self.restart_tracker.entry(id.clone()).or_insert_with(|| {
                RestartHistory {
                    attempts: Vec::new(),
                    total_restarts: 0,
                }
            });
            entry.attempts.push(Instant::now());
            entry.total_restarts = total + 1;
        }

        info!(plugin_id = %id, total_restarts = total + 1, "plugin restarted via policy");
        Ok(true)
    }

    /// Return the total number of restarts recorded for a plugin (across all time).
    pub fn restart_count(&self, id: &PluginId) -> u32 {
        self.restart_tracker
            .get(id)
            .map(|h| h.total_restarts)
            .unwrap_or(0)
    }

    /// Activate with a timeout. Returns error if activation takes too long.
    pub async fn activate_with_timeout(
        &self,
        id: &PluginId,
        timeout: std::time::Duration,
    ) -> Result<()> {
        match tokio::time::timeout(timeout, self.activate(id)).await {
            Ok(result) => result,
            Err(_) => Err(Error::PluginActivation {
                plugin_id: id.to_string(),
                reason: format!("activation timed out after {}ms", timeout.as_millis()),
            }),
        }
    }

    /// Unregister a plugin completely (remove from registry). Must be in Installed state.
    pub fn unregister(&self, id: &PluginId) -> Result<()> {
        let state = self.get_state(id)?;
        if state.is_active() {
            return Err(Error::Plugin {
                plugin_id: id.to_string(),
                message: "cannot unregister: plugin is still active, deactivate first".to_string(),
            });
        }
        self.plugins.remove(id);
        info!(plugin_id = %id, "plugin unregistered");
        Ok(())
    }

    /// Number of registered plugins.
    pub fn count(&self) -> usize {
        self.plugins.len()
    }

    /// Collect HTTP routers contributed by every active plugin.
    ///
    /// Walks the registry, locks each active plugin briefly, and calls
    /// `Plugin::http_router`. Plugins that return `None` (the default)
    /// are skipped. The returned routers are ready to be `merge`d into
    /// the admin-api Axum router — admin-api therefore needs no compile-
    /// time knowledge of any specific plugin crate.
    pub async fn collect_http_routers(&self) -> Vec<Router> {
        // Snapshot active plugin IDs first to avoid holding DashMap shard
        // locks across an `.await`.
        let active_ids: Vec<PluginId> = self
            .plugins
            .iter()
            .filter(|e| e.state.is_active())
            .map(|e| e.key().clone())
            .collect();

        let mut routers = Vec::new();
        for id in active_ids {
            if let Some(entry) = self.plugins.get(&id) {
                let plugin = entry.plugin.lock().await;
                if let Some(router) = plugin.http_router() {
                    debug!(plugin_id = %id, "collected HTTP router");
                    routers.push(router);
                }
            }
        }
        routers
    }

    /// Dispatch an incoming WebSocket request frame to the plugin whose
    /// `ws_action_prefix` matches the action.
    ///
    /// Walks active plugins, picks the longest matching prefix, locks the
    /// plugin briefly, and forwards the call. Returns `Err` if no active
    /// plugin claims the action.
    pub async fn dispatch_ws_action(
        &self,
        action: &str,
        payload: Value,
        ctx: WsActionContext,
    ) -> Result<Value> {
        // Snapshot active IDs to avoid holding shard locks across awaits.
        let active_ids: Vec<PluginId> = self
            .plugins
            .iter()
            .filter(|e| e.state.is_active())
            .map(|e| e.key().clone())
            .collect();

        // Longest-prefix match.
        let mut best: Option<(usize, PluginId)> = None;
        for id in &active_ids {
            if let Some(entry) = self.plugins.get(id) {
                let plugin = entry.plugin.lock().await;
                if let Some(prefix) = plugin.ws_action_prefix() {
                    if action.starts_with(prefix) {
                        let len = prefix.len();
                        if best.as_ref().map_or(true, |(b, _)| len > *b) {
                            best = Some((len, id.clone()));
                        }
                    }
                }
            }
        }

        let target_id = best.map(|(_, id)| id).ok_or_else(|| Error::Plugin {
            plugin_id: "unknown".to_string(),
            message:  format!("no active plugin handles action: {action}"),
        })?;

        let entry = self
            .plugins
            .get(&target_id)
            .ok_or_else(|| Error::PluginNotFound(target_id.to_string()))?;
        let plugin = entry.plugin.lock().await;
        plugin.handle_ws_action(action.to_string(), payload, ctx).await
    }

    /// Deactivate all active plugins (for graceful shutdown).
    pub async fn deactivate_all(&self) {
        let ids: Vec<PluginId> = self
            .plugins
            .iter()
            .filter(|e| e.state.is_active())
            .map(|e| e.key().clone())
            .collect();

        for id in ids {
            if let Err(e) = self.deactivate(&id).await {
                error!(plugin_id = %id, error = %e, "failed to deactivate plugin during shutdown");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plugin_sdk::traits::BoxFuture;
    use server_core::ServerInfo;

    // ── Test plugin ──────────────────────────────────

    struct TestPlugin {
        id: PluginId,
        name: String,
        activated: bool,
        enabled: bool,
    }

    impl TestPlugin {
        fn new(id: &str, name: &str) -> Self {
            Self {
                id: PluginId::from_str(id),
                name: name.to_string(),
                activated: false,
                enabled: false,
            }
        }
    }

    impl Plugin for TestPlugin {
        fn id(&self) -> &PluginId {
            &self.id
        }
        fn name(&self) -> &str {
            &self.name
        }
        fn version(&self) -> &str {
            "1.0.0"
        }

        fn activate(&mut self, _ctx: plugin_sdk::PluginContext) -> BoxFuture<'_, Result<()>> {
            Box::pin(async {
                self.activated = true;
                self.enabled = true;
                Ok(())
            })
        }

        fn deactivate(&mut self) -> BoxFuture<'_, Result<()>> {
            Box::pin(async {
                self.activated = false;
                self.enabled = false;
                Ok(())
            })
        }

        fn on_enable(&mut self) -> BoxFuture<'_, Result<()>> {
            Box::pin(async {
                self.enabled = true;
                Ok(())
            })
        }

        fn on_disable(&mut self) -> BoxFuture<'_, Result<()>> {
            Box::pin(async {
                self.enabled = false;
                Ok(())
            })
        }
    }

    fn make_registry() -> PluginRegistry {
        let bus = Arc::new(EventBus::new(16));
        let cache: Arc<dyn cache_layer::CacheBackend> = Arc::new(
            cache_layer::MemoryCache::new(&server_config::model::MemoryCacheConfig::default()),
        );
        let builder = ContextBuilder::new(ServerInfo::default(), Arc::clone(&bus), cache);
        PluginRegistry::new(builder, bus)
    }

    #[test]
    fn test_register_builtin() {
        let registry = make_registry();
        let plugin = TestPlugin::new("io.draox.test", "Test");
        registry.register_builtin(Box::new(plugin)).unwrap();
        assert_eq!(registry.count(), 1);

        let state = registry
            .get_state(&PluginId::from_str("io.draox.test"))
            .unwrap();
        assert_eq!(state, PluginState::Installed);
    }

    #[test]
    fn test_duplicate_register_fails() {
        let registry = make_registry();
        let p1 = TestPlugin::new("io.draox.test", "Test");
        let p2 = TestPlugin::new("io.draox.test", "Test Dup");
        registry.register_builtin(Box::new(p1)).unwrap();
        assert!(registry.register_builtin(Box::new(p2)).is_err());
    }

    #[tokio::test]
    async fn test_activate_plugin() {
        let registry = make_registry();
        let id = PluginId::from_str("io.draox.test");
        registry
            .register_builtin(Box::new(TestPlugin::new("io.draox.test", "Test")))
            .unwrap();

        registry.activate(&id).await.unwrap();
        assert_eq!(registry.get_state(&id).unwrap(), PluginState::ActiveEnabled);
    }

    #[tokio::test]
    async fn test_full_lifecycle() {
        let registry = make_registry();
        let id = PluginId::from_str("io.draox.test");
        registry
            .register_builtin(Box::new(TestPlugin::new("io.draox.test", "Test")))
            .unwrap();

        // Activate
        registry.activate(&id).await.unwrap();
        assert_eq!(registry.get_state(&id).unwrap(), PluginState::ActiveEnabled);

        // Disable
        registry.disable(&id).await.unwrap();
        assert_eq!(
            registry.get_state(&id).unwrap(),
            PluginState::ActiveDisabled
        );

        // Enable
        registry.enable(&id).await.unwrap();
        assert_eq!(registry.get_state(&id).unwrap(), PluginState::ActiveEnabled);

        // Deactivate
        registry.deactivate(&id).await.unwrap();
        assert_eq!(registry.get_state(&id).unwrap(), PluginState::Installed);
    }

    #[tokio::test]
    async fn test_invalid_transition() {
        let registry = make_registry();
        let id = PluginId::from_str("io.draox.test");
        registry
            .register_builtin(Box::new(TestPlugin::new("io.draox.test", "Test")))
            .unwrap();

        // Can't disable an installed plugin
        assert!(registry.disable(&id).await.is_err());
        // Can't enable an installed plugin
        assert!(registry.enable(&id).await.is_err());
    }

    #[tokio::test]
    async fn test_health_check() {
        let registry = make_registry();
        let id = PluginId::from_str("io.draox.test");
        registry
            .register_builtin(Box::new(TestPlugin::new("io.draox.test", "Test")))
            .unwrap();

        let health = registry.health_check(&id).await.unwrap();
        assert!(health.is_healthy());
    }

    #[tokio::test]
    async fn test_get_info() {
        let registry = make_registry();
        let id = PluginId::from_str("io.draox.test");
        registry
            .register_builtin(Box::new(TestPlugin::new("io.draox.test", "Test")))
            .unwrap();

        let info = registry.get_info(&id).await.unwrap();
        assert_eq!(info.name, "Test");
        assert_eq!(info.version, "1.0.0");
        assert_eq!(info.state, PluginState::Installed);
    }

    #[tokio::test]
    async fn test_deactivate_all() {
        let registry = make_registry();
        let id1 = PluginId::from_str("io.draox.p1");
        let id2 = PluginId::from_str("io.draox.p2");

        registry
            .register_builtin(Box::new(TestPlugin::new("io.draox.p1", "P1")))
            .unwrap();
        registry
            .register_builtin(Box::new(TestPlugin::new("io.draox.p2", "P2")))
            .unwrap();

        registry.activate(&id1).await.unwrap();
        registry.activate(&id2).await.unwrap();

        registry.deactivate_all().await;

        assert_eq!(registry.get_state(&id1).unwrap(), PluginState::Installed);
        assert_eq!(registry.get_state(&id2).unwrap(), PluginState::Installed);
    }

    #[test]
    fn test_list_plugins() {
        let registry = make_registry();
        registry
            .register_builtin(Box::new(TestPlugin::new("io.draox.a", "A")))
            .unwrap();
        registry
            .register_builtin(Box::new(TestPlugin::new("io.draox.b", "B")))
            .unwrap();

        let list = registry.list();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_restart_plugin() {
        let registry = make_registry();
        let id = PluginId::from_str("io.draox.test");
        registry
            .register_builtin(Box::new(TestPlugin::new("io.draox.test", "Test")))
            .unwrap();

        // Activate first
        registry.activate(&id).await.unwrap();
        assert_eq!(registry.get_state(&id).unwrap(), PluginState::ActiveEnabled);

        // Restart
        registry.restart(&id).await.unwrap();
        assert_eq!(registry.get_state(&id).unwrap(), PluginState::ActiveEnabled);
    }

    #[tokio::test]
    async fn test_restart_inactive_fails() {
        let registry = make_registry();
        let id = PluginId::from_str("io.draox.test");
        registry
            .register_builtin(Box::new(TestPlugin::new("io.draox.test", "Test")))
            .unwrap();

        // Plugin is in Installed state -- restart should fail
        let result = registry.restart(&id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_activate_with_timeout() {
        let registry = make_registry();
        let id = PluginId::from_str("io.draox.test");
        registry
            .register_builtin(Box::new(TestPlugin::new("io.draox.test", "Test")))
            .unwrap();

        // Activation should succeed well within the timeout
        let result = registry
            .activate_with_timeout(&id, std::time::Duration::from_secs(5))
            .await;
        assert!(result.is_ok());
        assert_eq!(registry.get_state(&id).unwrap(), PluginState::ActiveEnabled);
    }

    #[test]
    fn test_unregister_plugin() {
        let registry = make_registry();
        let id = PluginId::from_str("io.draox.test");
        registry
            .register_builtin(Box::new(TestPlugin::new("io.draox.test", "Test")))
            .unwrap();
        assert_eq!(registry.count(), 1);

        // Unregister (plugin is in Installed state)
        registry.unregister(&id).unwrap();
        assert_eq!(registry.count(), 0);
        assert!(registry.get_state(&id).is_err());
    }

    #[tokio::test]
    async fn test_unregister_active_fails() {
        let registry = make_registry();
        let id = PluginId::from_str("io.draox.test");
        registry
            .register_builtin(Box::new(TestPlugin::new("io.draox.test", "Test")))
            .unwrap();

        // Activate the plugin
        registry.activate(&id).await.unwrap();

        // Can't unregister an active plugin
        let result = registry.unregister(&id);
        assert!(result.is_err());
        // Plugin should still be there
        assert_eq!(registry.count(), 1);
    }

    // ── RestartPolicy tests ─────────────────────────────────────────────────

    #[tokio::test]
    async fn test_restart_with_policy_succeeds_first_time() {
        let registry = make_registry();
        let id = PluginId::from_str("io.draox.test");
        registry
            .register_builtin(Box::new(TestPlugin::new("io.draox.test", "Test")))
            .unwrap();
        registry.activate(&id).await.unwrap();

        // First restart should always be allowed.
        let policy = RestartPolicy {
            max_attempts: 3,
            cooldown: Duration::from_secs(60),
            backoff_multiplier: 2.0,
        };
        let result = registry.restart_with_policy(&id, &policy).await.unwrap();
        assert!(result, "first restart should succeed");
        assert_eq!(registry.restart_count(&id), 1);
        assert_eq!(registry.get_state(&id).unwrap(), PluginState::ActiveEnabled);
    }

    #[tokio::test]
    async fn test_restart_with_policy_max_attempts_blocked() {
        let registry = make_registry();
        let id = PluginId::from_str("io.draox.test");
        registry
            .register_builtin(Box::new(TestPlugin::new("io.draox.test", "Test")))
            .unwrap();
        registry.activate(&id).await.unwrap();

        // Use zero cooldown so window pruning doesn't remove records,
        // but allow max_attempts = 1 so the second call is blocked.
        let policy = RestartPolicy {
            max_attempts: 1,
            // A very long cooldown ensures the first attempt stays in-window.
            cooldown: Duration::from_secs(3600),
            backoff_multiplier: 1.0, // no extra backoff
        };

        // First restart is allowed.
        let r1 = registry.restart_with_policy(&id, &policy).await.unwrap();
        assert!(r1);

        // Second restart should be blocked because max_attempts=1 within the window.
        let r2 = registry.restart_with_policy(&id, &policy).await.unwrap();
        assert!(!r2, "second restart should be blocked by max_attempts");
    }

    #[tokio::test]
    async fn test_restart_with_policy_backoff_blocked() {
        let registry = make_registry();
        let id = PluginId::from_str("io.draox.test");
        registry
            .register_builtin(Box::new(TestPlugin::new("io.draox.test", "Test")))
            .unwrap();
        registry.activate(&id).await.unwrap();

        // Allow up to 5 attempts but require a 1-hour cooldown between each.
        let policy = RestartPolicy {
            max_attempts: 5,
            cooldown: Duration::from_secs(3600),
            backoff_multiplier: 2.0,
        };

        // First restart: no prior attempt, no backoff required → should proceed.
        let r1 = registry.restart_with_policy(&id, &policy).await.unwrap();
        assert!(r1);

        // Immediately try again — backoff of cooldown * multiplier^1 = 2 hours not elapsed.
        let r2 = registry.restart_with_policy(&id, &policy).await.unwrap();
        assert!(!r2, "second immediate restart should be blocked by backoff");
    }

    #[tokio::test]
    async fn test_restart_with_policy_inactive_plugin_errors() {
        let registry = make_registry();
        let id = PluginId::from_str("io.draox.test");
        registry
            .register_builtin(Box::new(TestPlugin::new("io.draox.test", "Test")))
            .unwrap();
        // Plugin is Installed (not active) — restart should return Err.
        let policy = RestartPolicy::default();
        let result = registry.restart_with_policy(&id, &policy).await;
        assert!(result.is_err(), "restarting inactive plugin should error");
    }

    #[test]
    fn test_restart_policy_default_values() {
        let policy = RestartPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.cooldown, Duration::from_secs(5));
        assert!((policy.backoff_multiplier - 2.0).abs() < f64::EPSILON);
    }
}
