use crate::marketplace_types::{
    MarketplacePlugin, PluginAnalytics, PluginCategory, PluginReview, PluginVersion, PublisherInfo,
    SearchQuery, SearchResult, SortBy,
};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::RwLock;

/// In-memory marketplace registry that stores and serves plugin metadata.
///
/// All operations are safe for concurrent use; `DashMap` provides per-shard
/// locking, and the `featured` list uses a `std::sync::RwLock`.
pub struct MarketplaceRegistry {
    plugins: DashMap<String, MarketplacePlugin>,
    /// plugin_id → ordered list of versions (newest last)
    versions: DashMap<String, Vec<PluginVersion>>,
    /// plugin_id → list of reviews
    reviews: DashMap<String, Vec<PluginReview>>,
    publishers: DashMap<String, PublisherInfo>,
    analytics: DashMap<String, PluginAnalytics>,
    /// Ordered list of featured plugin IDs.
    featured: RwLock<Vec<String>>,
}

impl Default for MarketplaceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MarketplaceRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            plugins: DashMap::new(),
            versions: DashMap::new(),
            reviews: DashMap::new(),
            publishers: DashMap::new(),
            analytics: DashMap::new(),
            featured: RwLock::new(Vec::new()),
        }
    }

    // ── Plugin management ────────────────────────────────────────────────────

    /// Publish a plugin.  Returns `Err` if a plugin with the same ID already exists.
    pub fn publish(&self, plugin: MarketplacePlugin) -> Result<(), String> {
        if self.plugins.contains_key(&plugin.id) {
            return Err(format!("plugin '{}' already published", plugin.id));
        }
        // Seed analytics entry.
        self.analytics.entry(plugin.id.clone()).or_insert_with(|| PluginAnalytics {
            plugin_id: plugin.id.clone(),
            total_downloads: 0,
            active_installs: 0,
            daily_downloads: Vec::new(),
            version_distribution: std::collections::HashMap::new(),
        });
        self.plugins.insert(plugin.id.clone(), plugin);
        Ok(())
    }

    /// Return a clone of a plugin's metadata by ID.
    pub fn get_plugin(&self, id: &str) -> Option<MarketplacePlugin> {
        self.plugins.get(id).map(|r| r.clone())
    }

    /// Remove a plugin and all associated data.  Returns `true` if it existed.
    pub fn remove_plugin(&self, id: &str) -> bool {
        if self.plugins.remove(id).is_some() {
            self.versions.remove(id);
            self.reviews.remove(id);
            self.analytics.remove(id);
            // Remove from featured list.
            if let Ok(mut f) = self.featured.write() {
                f.retain(|fid| fid != id);
            }
            true
        } else {
            false
        }
    }

    /// Replace the stored metadata for an existing plugin.  Returns `true` on success.
    pub fn update_plugin(&self, plugin: MarketplacePlugin) -> bool {
        if let Some(mut entry) = self.plugins.get_mut(&plugin.id) {
            *entry = plugin;
            true
        } else {
            false
        }
    }

    // ── Search & browse ──────────────────────────────────────────────────────

    /// Search the registry according to a [`SearchQuery`].
    pub fn search(&self, query: &SearchQuery) -> SearchResult {
        let mut results: Vec<MarketplacePlugin> = self
            .plugins
            .iter()
            .map(|r| r.clone())
            .filter(|p| {
                // Text match: id, name, description, tags
                if let Some(q) = &query.query {
                    let q = q.to_lowercase();
                    let haystack = format!(
                        "{} {} {} {}",
                        p.id.to_lowercase(),
                        p.name.to_lowercase(),
                        p.description.to_lowercase(),
                        p.tags.join(" ").to_lowercase()
                    );
                    if !haystack.contains(&q) {
                        return false;
                    }
                }
                // Category match
                if let Some(cat) = &query.category {
                    if &p.category != cat {
                        return false;
                    }
                }
                // Tag match (plugin must have ALL requested tags)
                if !query.tags.is_empty()
                    && !query.tags.iter().all(|t| p.tags.contains(t))
                {
                    return false;
                }
                true
            })
            .collect();

        // Sort
        match query.sort_by {
            SortBy::Downloads => results.sort_by(|a, b| b.downloads.cmp(&a.downloads)),
            SortBy::Rating => results
                .sort_by(|a, b| b.rating.partial_cmp(&a.rating).unwrap_or(std::cmp::Ordering::Equal)),
            SortBy::Newest => results.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
            SortBy::Updated => results.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
            SortBy::Relevance => {
                // For the in-memory registry, relevance falls back to download count.
                results.sort_by(|a, b| b.downloads.cmp(&a.downloads));
            }
        }

        let total = results.len() as u64;
        let page = query.page.max(1);
        let page_size = query.page_size.max(1);
        let start = ((page - 1) * page_size) as usize;
        let page_results = results.into_iter().skip(start).take(page_size as usize).collect();

        SearchResult {
            plugins: page_results,
            total,
            page,
            page_size,
        }
    }

    /// Return all plugins in a given category.
    pub fn list_by_category(&self, category: &PluginCategory) -> Vec<MarketplacePlugin> {
        self.plugins
            .iter()
            .filter(|r| &r.category == category)
            .map(|r| r.clone())
            .collect()
    }

    /// Return all featured plugins in order.
    pub fn list_featured(&self) -> Vec<MarketplacePlugin> {
        let ids = self.featured.read().unwrap_or_else(|p| p.into_inner()).clone();
        ids.iter()
            .filter_map(|id| self.plugins.get(id).map(|r| r.clone()))
            .collect()
    }

    /// Return the top `limit` plugins by download count.
    pub fn list_popular(&self, limit: usize) -> Vec<MarketplacePlugin> {
        let mut all: Vec<MarketplacePlugin> =
            self.plugins.iter().map(|r| r.clone()).collect();
        all.sort_by(|a, b| b.downloads.cmp(&a.downloads));
        all.truncate(limit);
        all
    }

    /// Return the `limit` newest plugins by `created_at`.
    pub fn list_newest(&self, limit: usize) -> Vec<MarketplacePlugin> {
        let mut all: Vec<MarketplacePlugin> =
            self.plugins.iter().map(|r| r.clone()).collect();
        all.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        all.truncate(limit);
        all
    }

    // ── Versions ─────────────────────────────────────────────────────────────

    /// Add a version for a plugin.  Returns `false` if the plugin is not registered.
    pub fn add_version(&self, plugin_id: &str, version: PluginVersion) -> bool {
        if !self.plugins.contains_key(plugin_id) {
            return false;
        }
        let mut versions = self.versions.entry(plugin_id.to_string()).or_default();
        versions.push(version);
        true
    }

    /// Return all versions for a plugin (oldest first as inserted).
    pub fn get_versions(&self, plugin_id: &str) -> Vec<PluginVersion> {
        self.versions
            .get(plugin_id)
            .map(|r| r.clone())
            .unwrap_or_default()
    }

    /// Return the most recently added version for a plugin.
    pub fn get_latest_version(&self, plugin_id: &str) -> Option<PluginVersion> {
        self.versions
            .get(plugin_id)
            .and_then(|r| r.last().cloned())
    }

    // ── Reviews & ratings ────────────────────────────────────────────────────

    /// Add a review.  Rating must be 1–5; returns `Err` otherwise.
    pub fn add_review(&self, review: PluginReview) -> Result<(), String> {
        if review.rating < 1 || review.rating > 5 {
            return Err(format!("invalid rating {}: must be 1–5", review.rating));
        }
        let plugin_id = review.plugin_id.clone();
        self.reviews
            .entry(plugin_id.clone())
            .or_default()
            .push(review);
        self.update_rating(&plugin_id);
        Ok(())
    }

    /// Return all reviews for a plugin.
    pub fn get_reviews(&self, plugin_id: &str) -> Vec<PluginReview> {
        self.reviews
            .get(plugin_id)
            .map(|r| r.clone())
            .unwrap_or_default()
    }

    /// Recalculate and store the average rating from all reviews.
    pub fn update_rating(&self, plugin_id: &str) {
        let reviews = self.get_reviews(plugin_id);
        if reviews.is_empty() {
            return;
        }
        let sum: f64 = reviews.iter().map(|r| r.rating as f64).sum();
        let avg = sum / reviews.len() as f64;
        // Round to one decimal.
        let avg = (avg * 10.0).round() / 10.0;
        if let Some(mut p) = self.plugins.get_mut(plugin_id) {
            p.rating = avg;
            p.rating_count = reviews.len() as u32;
        }
    }

    // ── Publishers ───────────────────────────────────────────────────────────

    /// Register a new publisher.  Returns `Err` if the ID is already in use.
    pub fn register_publisher(&self, publisher: PublisherInfo) -> Result<(), String> {
        if self.publishers.contains_key(&publisher.id) {
            return Err(format!("publisher '{}' already registered", publisher.id));
        }
        self.publishers.insert(publisher.id.clone(), publisher);
        Ok(())
    }

    /// Return a publisher by ID.
    pub fn get_publisher(&self, id: &str) -> Option<PublisherInfo> {
        self.publishers.get(id).map(|r| r.clone())
    }

    /// Mark a publisher as verified.  Returns `true` if the publisher exists.
    pub fn verify_publisher(&self, id: &str) -> bool {
        if let Some(mut p) = self.publishers.get_mut(id) {
            p.verified = true;
            true
        } else {
            false
        }
    }

    // ── Analytics ────────────────────────────────────────────────────────────

    /// Record a download event for a plugin.
    pub fn record_download(&self, plugin_id: &str) {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        if let Some(mut a) = self.analytics.get_mut(plugin_id) {
            a.total_downloads += 1;
            // Update daily count
            if let Some(entry) = a.daily_downloads.iter_mut().find(|(d, _)| d == &today) {
                entry.1 += 1;
            } else {
                a.daily_downloads.push((today, 1));
            }
        }
        // Mirror on plugin metadata
        if let Some(mut p) = self.plugins.get_mut(plugin_id) {
            p.downloads += 1;
        }
    }

    /// Record an install event for a plugin.
    pub fn record_install(&self, plugin_id: &str) {
        if let Some(mut a) = self.analytics.get_mut(plugin_id) {
            a.active_installs += 1;
        }
        if let Some(mut p) = self.plugins.get_mut(plugin_id) {
            p.active_installs += 1;
        }
    }

    /// Record an uninstall event for a plugin.
    pub fn record_uninstall(&self, plugin_id: &str) {
        if let Some(mut a) = self.analytics.get_mut(plugin_id) {
            if a.active_installs > 0 {
                a.active_installs -= 1;
            }
        }
        if let Some(mut p) = self.plugins.get_mut(plugin_id) {
            if p.active_installs > 0 {
                p.active_installs -= 1;
            }
        }
    }

    /// Return analytics for a plugin.
    pub fn get_analytics(&self, plugin_id: &str) -> Option<PluginAnalytics> {
        self.analytics.get(plugin_id).map(|r| r.clone())
    }

    // ── Featured ─────────────────────────────────────────────────────────────

    /// Replace the ordered list of featured plugin IDs.
    pub fn set_featured(&self, plugin_ids: Vec<String>) {
        if let Ok(mut f) = self.featured.write() {
            *f = plugin_ids;
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_publisher(id: &str) -> PublisherInfo {
        PublisherInfo {
            id: id.to_string(),
            name: format!("Publisher {id}"),
            email: None,
            verified: false,
            joined_at: Utc::now(),
        }
    }

    fn make_plugin(id: &str, category: PluginCategory, downloads: u64) -> MarketplacePlugin {
        MarketplacePlugin {
            id: id.to_string(),
            name: format!("Plugin {id}"),
            description: format!("Description for {id}"),
            version: "1.0.0".to_string(),
            author: make_publisher("pub-1"),
            category,
            tags: vec!["test".to_string()],
            downloads,
            active_installs: 0,
            rating: 0.0,
            rating_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            is_paid: false,
            price_cents: None,
            featured: false,
            compatibility: ">=1.0.0".to_string(),
        }
    }

    fn make_version(ver: &str) -> PluginVersion {
        PluginVersion {
            version: ver.to_string(),
            release_notes: "Initial release".to_string(),
            min_server_version: "1.0.0".to_string(),
            published_at: Utc::now(),
            download_url: format!("https://example.com/plugin-{ver}.dxp"),
            checksum: "abc123".to_string(),
            size_bytes: 1024,
            dependencies: vec![],
        }
    }

    // ── Plugin management ────────────────────────────────────────────────────

    #[test]
    fn test_publish_and_get_plugin() {
        let registry = MarketplaceRegistry::new();
        let plugin = make_plugin("io.draox.a", PluginCategory::Utility, 0);
        registry.publish(plugin.clone()).unwrap();
        let got = registry.get_plugin("io.draox.a").unwrap();
        assert_eq!(got.id, "io.draox.a");
    }

    #[test]
    fn test_publish_duplicate_fails() {
        let registry = MarketplaceRegistry::new();
        registry
            .publish(make_plugin("io.draox.a", PluginCategory::Utility, 0))
            .unwrap();
        assert!(registry
            .publish(make_plugin("io.draox.a", PluginCategory::Utility, 0))
            .is_err());
    }

    #[test]
    fn test_remove_plugin() {
        let registry = MarketplaceRegistry::new();
        registry
            .publish(make_plugin("io.draox.a", PluginCategory::Utility, 0))
            .unwrap();
        assert!(registry.remove_plugin("io.draox.a"));
        assert!(registry.get_plugin("io.draox.a").is_none());
        // Removing again returns false
        assert!(!registry.remove_plugin("io.draox.a"));
    }

    #[test]
    fn test_update_plugin() {
        let registry = MarketplaceRegistry::new();
        registry
            .publish(make_plugin("io.draox.a", PluginCategory::Utility, 0))
            .unwrap();
        let mut updated = make_plugin("io.draox.a", PluginCategory::Security, 9_000);
        updated.name = "Updated Name".to_string();
        assert!(registry.update_plugin(updated));
        let got = registry.get_plugin("io.draox.a").unwrap();
        assert_eq!(got.name, "Updated Name");
        assert_eq!(got.downloads, 9_000);
    }

    // ── Search & browse ──────────────────────────────────────────────────────

    #[test]
    fn test_search_by_text() {
        let registry = MarketplaceRegistry::new();
        registry
            .publish(make_plugin("io.draox.alpha", PluginCategory::Utility, 10))
            .unwrap();
        registry
            .publish(make_plugin("io.draox.beta", PluginCategory::Security, 5))
            .unwrap();

        let q = SearchQuery {
            query: Some("alpha".to_string()),
            ..Default::default()
        };
        let result = registry.search(&q);
        assert_eq!(result.total, 1);
        assert_eq!(result.plugins[0].id, "io.draox.alpha");
    }

    #[test]
    fn test_search_by_category() {
        let registry = MarketplaceRegistry::new();
        registry
            .publish(make_plugin("io.draox.a", PluginCategory::Analytics, 0))
            .unwrap();
        registry
            .publish(make_plugin("io.draox.b", PluginCategory::Utility, 0))
            .unwrap();

        let q = SearchQuery {
            category: Some(PluginCategory::Analytics),
            ..Default::default()
        };
        let result = registry.search(&q);
        assert_eq!(result.total, 1);
        assert_eq!(result.plugins[0].id, "io.draox.a");
    }

    #[test]
    fn test_list_popular() {
        let registry = MarketplaceRegistry::new();
        registry
            .publish(make_plugin("io.draox.low", PluginCategory::Utility, 100))
            .unwrap();
        registry
            .publish(make_plugin("io.draox.high", PluginCategory::Utility, 9_000))
            .unwrap();
        registry
            .publish(make_plugin("io.draox.mid", PluginCategory::Utility, 500))
            .unwrap();

        let popular = registry.list_popular(2);
        assert_eq!(popular.len(), 2);
        assert_eq!(popular[0].id, "io.draox.high");
        assert_eq!(popular[1].id, "io.draox.mid");
    }

    // ── Versions ─────────────────────────────────────────────────────────────

    #[test]
    fn test_add_and_get_versions() {
        let registry = MarketplaceRegistry::new();
        registry
            .publish(make_plugin("io.draox.a", PluginCategory::Utility, 0))
            .unwrap();
        assert!(registry.add_version("io.draox.a", make_version("1.0.0")));
        assert!(registry.add_version("io.draox.a", make_version("1.1.0")));

        let versions = registry.get_versions("io.draox.a");
        assert_eq!(versions.len(), 2);
    }

    #[test]
    fn test_get_latest_version() {
        let registry = MarketplaceRegistry::new();
        registry
            .publish(make_plugin("io.draox.a", PluginCategory::Utility, 0))
            .unwrap();
        registry.add_version("io.draox.a", make_version("1.0.0"));
        registry.add_version("io.draox.a", make_version("2.0.0"));

        let latest = registry.get_latest_version("io.draox.a").unwrap();
        assert_eq!(latest.version, "2.0.0");
    }

    #[test]
    fn test_add_version_unknown_plugin_returns_false() {
        let registry = MarketplaceRegistry::new();
        assert!(!registry.add_version("io.draox.nonexistent", make_version("1.0.0")));
    }

    // ── Reviews & ratings ────────────────────────────────────────────────────

    #[test]
    fn test_add_review_and_update_rating() {
        let registry = MarketplaceRegistry::new();
        registry
            .publish(make_plugin("io.draox.a", PluginCategory::Utility, 0))
            .unwrap();

        let review = PluginReview {
            id: "rev-1".to_string(),
            plugin_id: "io.draox.a".to_string(),
            reviewer: "alice".to_string(),
            rating: 5,
            comment: "Great!".to_string(),
            created_at: Utc::now(),
        };
        registry.add_review(review).unwrap();

        let got = registry.get_plugin("io.draox.a").unwrap();
        assert_eq!(got.rating_count, 1);
        assert!((got.rating - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_invalid_rating_rejected() {
        let registry = MarketplaceRegistry::new();
        registry
            .publish(make_plugin("io.draox.a", PluginCategory::Utility, 0))
            .unwrap();

        let bad = PluginReview {
            id: "rev-bad".to_string(),
            plugin_id: "io.draox.a".to_string(),
            reviewer: "bob".to_string(),
            rating: 6,
            comment: "Out of range".to_string(),
            created_at: Utc::now(),
        };
        assert!(registry.add_review(bad).is_err());
    }

    // ── Publishers ───────────────────────────────────────────────────────────

    #[test]
    fn test_register_and_verify_publisher() {
        let registry = MarketplaceRegistry::new();
        let pub_info = make_publisher("pub-x");
        registry.register_publisher(pub_info).unwrap();

        let p = registry.get_publisher("pub-x").unwrap();
        assert!(!p.verified);

        assert!(registry.verify_publisher("pub-x"));
        let p2 = registry.get_publisher("pub-x").unwrap();
        assert!(p2.verified);
    }

    // ── Analytics ────────────────────────────────────────────────────────────

    #[test]
    fn test_analytics_download_and_install() {
        let registry = MarketplaceRegistry::new();
        registry
            .publish(make_plugin("io.draox.a", PluginCategory::Utility, 0))
            .unwrap();

        registry.record_download("io.draox.a");
        registry.record_download("io.draox.a");
        registry.record_install("io.draox.a");
        registry.record_uninstall("io.draox.a");

        let analytics = registry.get_analytics("io.draox.a").unwrap();
        assert_eq!(analytics.total_downloads, 2);
        assert_eq!(analytics.active_installs, 0);

        let plugin = registry.get_plugin("io.draox.a").unwrap();
        assert_eq!(plugin.downloads, 2);
        assert_eq!(plugin.active_installs, 0);
    }

    // ── Featured ─────────────────────────────────────────────────────────────

    #[test]
    fn test_set_and_list_featured() {
        let registry = MarketplaceRegistry::new();
        registry
            .publish(make_plugin("io.draox.a", PluginCategory::Utility, 0))
            .unwrap();
        registry
            .publish(make_plugin("io.draox.b", PluginCategory::Security, 0))
            .unwrap();

        registry.set_featured(vec!["io.draox.a".to_string(), "io.draox.b".to_string()]);
        let featured = registry.list_featured();
        assert_eq!(featured.len(), 2);
        assert_eq!(featured[0].id, "io.draox.a");
    }

    #[test]
    fn test_remove_clears_from_featured() {
        let registry = MarketplaceRegistry::new();
        registry
            .publish(make_plugin("io.draox.a", PluginCategory::Utility, 0))
            .unwrap();
        registry.set_featured(vec!["io.draox.a".to_string()]);
        registry.remove_plugin("io.draox.a");
        assert!(registry.list_featured().is_empty());
    }
}
