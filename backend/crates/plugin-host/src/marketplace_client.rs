use crate::marketplace_registry::MarketplaceRegistry;
use crate::marketplace_types::{
    MarketplacePlugin, PluginReview, PluginVersion, SearchQuery, SearchResult,
};
use std::sync::Arc;

/// Full-featured marketplace client for interacting with a marketplace registry.
///
/// In production this would issue HTTP requests to `registry_url`.  For the
/// current implementation a *local mode* is provided that delegates directly to
/// an in-memory [`MarketplaceRegistry`], making the client fully testable
/// without network access.
pub struct RegistryClient {
    /// Remote registry URL (used in production mode).
    pub registry_url: String,
    /// Optional in-memory registry used instead of HTTP when set.
    local_registry: Option<Arc<MarketplaceRegistry>>,
}

impl RegistryClient {
    /// Create a client pointing at a remote registry URL.
    ///
    /// In the current implementation all remote calls return an error; use
    /// [`RegistryClient::with_local_registry`] for a working client.
    pub fn new(registry_url: &str) -> Self {
        Self {
            registry_url: registry_url.to_string(),
            local_registry: None,
        }
    }

    /// Create a client backed by an in-memory registry (local / test mode).
    pub fn with_local_registry(registry: Arc<MarketplaceRegistry>) -> Self {
        Self {
            registry_url: "local://".to_string(),
            local_registry: Some(registry),
        }
    }

    // ── Internal helpers ─────────────────────────────────────────────────────

    fn local(&self) -> Result<&MarketplaceRegistry, String> {
        self.local_registry
            .as_deref()
            .ok_or_else(|| "remote marketplace not yet implemented — use with_local_registry".to_string())
    }

    // ── Public async API ─────────────────────────────────────────────────────

    /// Search the marketplace.
    pub async fn search(&self, query: &SearchQuery) -> Result<SearchResult, String> {
        Ok(self.local()?.search(query))
    }

    /// Fetch metadata for a single plugin by ID.
    pub async fn get_plugin(&self, id: &str) -> Result<Option<MarketplacePlugin>, String> {
        Ok(self.local()?.get_plugin(id))
    }

    /// Fetch all published versions for a plugin.
    pub async fn get_versions(&self, id: &str) -> Result<Vec<PluginVersion>, String> {
        Ok(self.local()?.get_versions(id))
    }

    /// Fetch all reviews for a plugin.
    pub async fn get_reviews(&self, id: &str) -> Result<Vec<PluginReview>, String> {
        Ok(self.local()?.get_reviews(id))
    }

    /// Fetch the current list of featured plugins.
    pub async fn get_featured(&self) -> Result<Vec<MarketplacePlugin>, String> {
        Ok(self.local()?.list_featured())
    }

    /// Fetch the top `limit` plugins by download count.
    pub async fn get_popular(&self, limit: usize) -> Result<Vec<MarketplacePlugin>, String> {
        Ok(self.local()?.list_popular(limit))
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::marketplace_types::{
        PluginCategory, PublisherInfo, PluginVersion,
    };
    use chrono::Utc;

    fn make_registry_with_plugins() -> Arc<MarketplaceRegistry> {
        let reg = Arc::new(MarketplaceRegistry::new());

        let publisher = PublisherInfo {
            id: "pub-1".to_string(),
            name: "Test Publisher".to_string(),
            email: None,
            verified: true,
            joined_at: Utc::now(),
        };

        let plugins = vec![
            MarketplacePlugin {
                id: "io.draox.alpha".to_string(),
                name: "Alpha".to_string(),
                description: "Alpha plugin".to_string(),
                version: "1.0.0".to_string(),
                author: publisher.clone(),
                category: PluginCategory::Utility,
                tags: vec!["utility".to_string()],
                downloads: 5_000,
                active_installs: 1_000,
                rating: 4.2,
                rating_count: 10,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                is_paid: false,
                price_cents: None,
                featured: false,
                compatibility: ">=1.0.0".to_string(),
            },
            MarketplacePlugin {
                id: "io.draox.beta".to_string(),
                name: "Beta".to_string(),
                description: "Beta plugin".to_string(),
                version: "2.0.0".to_string(),
                author: publisher.clone(),
                category: PluginCategory::Security,
                tags: vec!["security".to_string()],
                downloads: 12_000,
                active_installs: 3_500,
                rating: 4.8,
                rating_count: 200,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                is_paid: false,
                price_cents: None,
                featured: true,
                compatibility: ">=1.0.0".to_string(),
            },
        ];

        for p in plugins {
            reg.publish(p).unwrap();
        }

        // Add a version to alpha
        reg.add_version("io.draox.alpha", PluginVersion {
            version: "1.0.0".to_string(),
            release_notes: "First release".to_string(),
            min_server_version: "1.0.0".to_string(),
            published_at: Utc::now(),
            download_url: "https://example.com/alpha-1.0.0.dxp".to_string(),
            checksum: "abc123".to_string(),
            size_bytes: 2048,
            dependencies: vec![],
        });

        reg.set_featured(vec!["io.draox.beta".to_string()]);

        reg
    }

    #[tokio::test]
    async fn test_client_search() {
        let registry = make_registry_with_plugins();
        let client = RegistryClient::with_local_registry(registry);

        let query = SearchQuery {
            query: Some("alpha".to_string()),
            ..Default::default()
        };
        let result = client.search(&query).await.unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.plugins[0].id, "io.draox.alpha");
    }

    #[tokio::test]
    async fn test_client_get_plugin() {
        let registry = make_registry_with_plugins();
        let client = RegistryClient::with_local_registry(registry);

        let plugin = client.get_plugin("io.draox.beta").await.unwrap();
        assert!(plugin.is_some());
        assert_eq!(plugin.unwrap().name, "Beta");

        let missing = client.get_plugin("io.draox.nonexistent").await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_client_get_versions() {
        let registry = make_registry_with_plugins();
        let client = RegistryClient::with_local_registry(registry);

        let versions = client.get_versions("io.draox.alpha").await.unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version, "1.0.0");
    }

    #[tokio::test]
    async fn test_client_get_featured() {
        let registry = make_registry_with_plugins();
        let client = RegistryClient::with_local_registry(registry);

        let featured = client.get_featured().await.unwrap();
        assert_eq!(featured.len(), 1);
        assert_eq!(featured[0].id, "io.draox.beta");
    }

    #[tokio::test]
    async fn test_client_get_popular() {
        let registry = make_registry_with_plugins();
        let client = RegistryClient::with_local_registry(registry);

        let popular = client.get_popular(1).await.unwrap();
        assert_eq!(popular.len(), 1);
        // beta has more downloads (12,000 vs 5,000)
        assert_eq!(popular[0].id, "io.draox.beta");
    }

    #[tokio::test]
    async fn test_client_remote_not_implemented() {
        let client = RegistryClient::new("https://marketplace.draox-server.io");
        let result = client.get_plugin("io.draox.test").await;
        assert!(result.is_err());
    }
}
