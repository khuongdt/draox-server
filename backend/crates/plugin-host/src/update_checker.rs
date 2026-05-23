use crate::marketplace_client::RegistryClient;
use crate::marketplace_types::PluginVersion;
use crate::version_resolver::VersionResolver;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info};

/// Periodically checks for available updates for installed plugins.
pub struct UpdateChecker {
    client: Arc<RegistryClient>,
    /// plugin_id → currently installed version string.
    installed: DashMap<String, String>,
    /// plugin_id → latest available `PluginVersion` (populated after a check).
    available_updates: DashMap<String, PluginVersion>,
    /// Interval between automatic update checks.
    pub check_interval: Duration,
}

impl UpdateChecker {
    /// Create a new `UpdateChecker` backed by `client`.
    pub fn new(client: Arc<RegistryClient>, interval: Duration) -> Self {
        Self {
            client,
            installed: DashMap::new(),
            available_updates: DashMap::new(),
            check_interval: interval,
        }
    }

    /// Register a plugin as installed at `version`.
    pub fn register_installed(&self, plugin_id: &str, version: &str) {
        self.installed.insert(plugin_id.to_string(), version.to_string());
        // Clear any cached update info when the installed version changes.
        self.available_updates.remove(plugin_id);
        debug!(plugin_id, version, "registered installed plugin version");
    }

    /// Remove a plugin from tracking.
    pub fn unregister(&self, plugin_id: &str) {
        self.installed.remove(plugin_id);
        self.available_updates.remove(plugin_id);
        debug!(plugin_id, "unregistered plugin from update checker");
    }

    /// Check all registered plugins for available updates.
    ///
    /// Returns a list of `(plugin_id, current_version, latest_version)` for
    /// every plugin that has a newer version available.
    pub async fn check_updates(&self) -> Vec<(String, String, String)> {
        let mut updates = Vec::new();

        let ids: Vec<(String, String)> = self
            .installed
            .iter()
            .map(|r| (r.key().clone(), r.value().clone()))
            .collect();

        for (plugin_id, installed_ver) in ids {
            match self.client.get_versions(&plugin_id).await {
                Ok(versions) if !versions.is_empty() => {
                    let available: Vec<&str> =
                        versions.iter().map(|v| v.version.as_str()).collect();

                    // Find the best version that is strictly newer than the installed one.
                    let requirement = format!(">{installed_ver}");
                    if let Some(best_str) =
                        VersionResolver::resolve_best(&available, &requirement)
                    {
                        // Retrieve the full PluginVersion metadata.
                        if let Some(best_ver) =
                            versions.iter().find(|v| v.version == best_str)
                        {
                            info!(
                                plugin_id,
                                current = %installed_ver,
                                latest = %best_str,
                                "update available"
                            );
                            self.available_updates
                                .insert(plugin_id.clone(), best_ver.clone());
                            updates.push((plugin_id, installed_ver, best_str));
                        }
                    }
                }
                Ok(_) => {
                    // No versions returned — nothing to do.
                }
                Err(e) => {
                    debug!(plugin_id, error = %e, "failed to fetch versions during update check");
                }
            }
        }

        updates
    }

    /// Return all plugins that have a known update available (as populated by
    /// the last [`check_updates`] call).
    pub fn get_available_updates(&self) -> Vec<(String, PluginVersion)> {
        self.available_updates
            .iter()
            .map(|r| (r.key().clone(), r.value().clone()))
            .collect()
    }

    /// Return `true` if the last update check found an update for `plugin_id`.
    pub fn has_update(&self, plugin_id: &str) -> bool {
        self.available_updates.contains_key(plugin_id)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::marketplace_registry::MarketplaceRegistry;
    use crate::marketplace_types::{
        MarketplacePlugin, PluginCategory, PublisherInfo,
    };
    use chrono::Utc;

    fn make_registry() -> Arc<MarketplaceRegistry> {
        Arc::new(MarketplaceRegistry::new())
    }

    fn make_client(registry: Arc<MarketplaceRegistry>) -> Arc<RegistryClient> {
        Arc::new(RegistryClient::with_local_registry(registry))
    }

    fn add_plugin(registry: &MarketplaceRegistry, id: &str, versions: &[&str]) {
        let plugin = MarketplacePlugin {
            id: id.to_string(),
            name: id.to_string(),
            description: String::new(),
            version: versions.last().copied().unwrap_or("1.0.0").to_string(),
            author: PublisherInfo {
                id: "pub".to_string(),
                name: "Test Publisher".to_string(),
                email: None,
                verified: true,
                joined_at: Utc::now(),
            },
            category: PluginCategory::Utility,
            tags: vec![],
            downloads: 0,
            active_installs: 0,
            rating: 0.0,
            rating_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            is_paid: false,
            price_cents: None,
            featured: false,
            compatibility: ">=1.0.0".to_string(),
        };
        registry.publish(plugin).unwrap();

        for ver in versions {
            registry.add_version(
                id,
                PluginVersion {
                    version: ver.to_string(),
                    release_notes: format!("Release {ver}"),
                    min_server_version: "1.0.0".to_string(),
                    published_at: Utc::now(),
                    download_url: format!("https://example.com/{id}-{ver}.dxp"),
                    checksum: "abc".to_string(),
                    size_bytes: 1024,
                    dependencies: vec![],
                },
            );
        }
    }

    #[test]
    fn test_register_and_unregister() {
        let registry = make_registry();
        let client = make_client(Arc::clone(&registry));
        let checker = UpdateChecker::new(client, Duration::from_secs(60));

        checker.register_installed("io.draox.a", "1.0.0");
        assert!(checker.installed.contains_key("io.draox.a"));

        checker.unregister("io.draox.a");
        assert!(!checker.installed.contains_key("io.draox.a"));
    }

    #[tokio::test]
    async fn test_check_updates_finds_newer_version() {
        let registry = make_registry();
        add_plugin(&registry, "io.draox.a", &["1.0.0", "1.1.0", "2.0.0"]);

        let client = make_client(Arc::clone(&registry));
        let checker = UpdateChecker::new(Arc::clone(&client), Duration::from_secs(60));

        // Installed version is 1.0.0; 2.0.0 should be detected as an update.
        checker.register_installed("io.draox.a", "1.0.0");
        let updates = checker.check_updates().await;

        assert_eq!(updates.len(), 1);
        let (id, current, latest) = &updates[0];
        assert_eq!(id, "io.draox.a");
        assert_eq!(current, "1.0.0");
        assert_eq!(latest, "2.0.0");
        assert!(checker.has_update("io.draox.a"));
    }

    #[tokio::test]
    async fn test_check_updates_no_update_when_up_to_date() {
        let registry = make_registry();
        add_plugin(&registry, "io.draox.b", &["1.0.0"]);

        let client = make_client(Arc::clone(&registry));
        let checker = UpdateChecker::new(Arc::clone(&client), Duration::from_secs(60));

        checker.register_installed("io.draox.b", "1.0.0");
        let updates = checker.check_updates().await;

        assert!(updates.is_empty());
        assert!(!checker.has_update("io.draox.b"));
    }

    #[tokio::test]
    async fn test_get_available_updates_after_check() {
        let registry = make_registry();
        add_plugin(&registry, "io.draox.c", &["1.0.0", "1.5.0"]);

        let client = make_client(Arc::clone(&registry));
        let checker = UpdateChecker::new(Arc::clone(&client), Duration::from_secs(60));

        checker.register_installed("io.draox.c", "1.0.0");
        checker.check_updates().await;

        let available = checker.get_available_updates();
        assert_eq!(available.len(), 1);
        assert_eq!(available[0].0, "io.draox.c");
        assert_eq!(available[0].1.version, "1.5.0");
    }
}
