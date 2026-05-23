use crate::marketplace_types::{
    MarketplacePlugin, PluginAnalytics, PluginReview, PluginVersion, PublisherInfo, SearchQuery,
    SearchResult, SortBy,
};
use dashmap::DashMap;

/// Lightweight marketplace entry used as a catalogue stub (for simple sync tasks).
///
/// For full plugin details see [`MarketplacePlugin`] in `marketplace_types`.
#[derive(Debug, Clone)]
pub struct MarketplaceEntry {
    /// Reverse-domain plugin identifier (e.g. "io.draox.clans").
    pub plugin_id: String,
    /// Human-readable plugin name.
    pub name: String,
    /// Semver version string (e.g. "1.2.3").
    pub version: String,
    /// Short description shown in the marketplace listing.
    pub description: Option<String>,
    /// Download URL for the `.dxp` package.
    pub download_url: String,
    /// SHA-256 hex digest of the `.dxp` package for integrity verification.
    pub checksum_sha256: String,
}

/// Local registry that mirrors the remote marketplace catalogue.
///
/// Acts as a read-through cache: the `admin-api` queries this registry to serve
/// marketplace listings without hitting the remote API on every request.
/// Background tasks are responsible for refreshing entries.
pub struct MarketplaceRegistry {
    /// plugin_id → lightweight entry (legacy / quick look-ups)
    entries: DashMap<String, MarketplaceEntry>,
    /// plugin_id → full plugin record
    plugins: DashMap<String, MarketplacePlugin>,
    /// plugin_id → released versions
    versions: DashMap<String, Vec<PluginVersion>>,
    /// plugin_id → submitted reviews
    reviews: DashMap<String, Vec<PluginReview>>,
    /// publisher_id → publisher profile
    publishers: DashMap<String, PublisherInfo>,
    /// plugin_id → install/download analytics
    analytics: DashMap<String, PluginAnalytics>,
}

impl MarketplaceRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            entries: DashMap::new(),
            plugins: DashMap::new(),
            versions: DashMap::new(),
            reviews: DashMap::new(),
            publishers: DashMap::new(),
            analytics: DashMap::new(),
        }
    }

    // ── Legacy entry API (lightweight) ────────────────────────────────────────

    /// Insert or update a lightweight marketplace entry.
    pub fn upsert(&self, entry: MarketplaceEntry) {
        self.entries.insert(entry.plugin_id.clone(), entry);
    }

    /// Remove a lightweight entry by plugin ID. Returns `true` if it existed.
    pub fn remove(&self, plugin_id: &str) -> bool {
        self.entries.remove(plugin_id).is_some()
    }

    /// All lightweight entries currently cached in the registry.
    pub fn all(&self) -> Vec<MarketplaceEntry> {
        self.entries.iter().map(|e| e.value().clone()).collect()
    }

    /// Number of lightweight entries in the registry.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the registry holds no lightweight entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Retrieve a lightweight entry by plugin ID.
    pub fn get_entry(&self, plugin_id: &str) -> Option<MarketplaceEntry> {
        self.entries.get(plugin_id).map(|e| e.clone())
    }

    // ── Full plugin API ───────────────────────────────────────────────────────

    /// Insert or replace a full plugin record.
    pub fn upsert_plugin(&self, plugin: MarketplacePlugin) {
        self.plugins.insert(plugin.id.clone(), plugin);
    }

    /// Retrieve a single full plugin record by ID.
    pub fn get(&self, plugin_id: &str) -> Option<MarketplacePlugin> {
        self.plugins.get(plugin_id).map(|p| p.clone())
    }

    /// Register a new plugin (publish workflow). Returns `Err` if already published.
    pub fn publish(&self, plugin: MarketplacePlugin) -> Result<(), String> {
        if self.plugins.contains_key(&plugin.id) {
            return Err(format!("plugin '{}' already published", plugin.id));
        }
        self.plugins.insert(plugin.id.clone(), plugin);
        Ok(())
    }

    // ── Search ────────────────────────────────────────────────────────────────

    /// Search the full plugin catalogue with filtering and sorting.
    pub fn search(&self, query: &SearchQuery) -> SearchResult {
        let page = query.page.max(1);
        let page_size = query.page_size.max(1).min(100);

        let mut results: Vec<MarketplacePlugin> = self
            .plugins
            .iter()
            .filter(|entry| {
                let p = entry.value();
                // Category filter
                if let Some(cat) = &query.category {
                    if &p.category != cat {
                        return false;
                    }
                }
                // Text search
                if let Some(q) = &query.query {
                    let q_lower = q.to_lowercase();
                    if !p.name.to_lowercase().contains(&q_lower)
                        && !p.description.to_lowercase().contains(&q_lower)
                        && !p.tags.iter().any(|t| t.to_lowercase().contains(&q_lower))
                    {
                        return false;
                    }
                }
                // Tags filter
                if !query.tags.is_empty()
                    && !query
                        .tags
                        .iter()
                        .any(|tag| p.tags.iter().any(|t| t == tag))
                {
                    return false;
                }
                true
            })
            .map(|entry| entry.value().clone())
            .collect();

        // Sort
        match query.sort_by {
            SortBy::Downloads => results.sort_by(|a, b| b.downloads.cmp(&a.downloads)),
            SortBy::Rating => results.sort_by(|a, b| {
                b.rating
                    .partial_cmp(&a.rating)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            SortBy::Newest => results.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
            SortBy::Updated => results.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
            SortBy::Relevance => results.sort_by(|a, b| a.name.cmp(&b.name)),
        }

        let total = results.len() as u64;
        let offset = ((page - 1) * page_size) as usize;
        let plugins = results
            .into_iter()
            .skip(offset)
            .take(page_size as usize)
            .collect();

        SearchResult {
            plugins,
            total,
            page,
            page_size,
        }
    }

    // ── Featured / Popular ────────────────────────────────────────────────────

    /// All plugins flagged as featured.
    pub fn featured(&self) -> Vec<MarketplacePlugin> {
        self.plugins
            .iter()
            .filter(|e| e.value().featured)
            .map(|e| e.value().clone())
            .collect()
    }

    /// Top plugins by download count.
    pub fn popular(&self, limit: usize) -> Vec<MarketplacePlugin> {
        let mut all: Vec<MarketplacePlugin> =
            self.plugins.iter().map(|e| e.value().clone()).collect();
        all.sort_by(|a, b| b.downloads.cmp(&a.downloads));
        all.truncate(limit);
        all
    }

    // ── Versions ──────────────────────────────────────────────────────────────

    pub fn get_versions(&self, plugin_id: &str) -> Vec<PluginVersion> {
        self.versions
            .get(plugin_id)
            .map(|v| v.clone())
            .unwrap_or_default()
    }

    pub fn add_version(&self, plugin_id: &str, version: PluginVersion) {
        self.versions
            .entry(plugin_id.to_string())
            .or_default()
            .push(version);
    }

    // ── Reviews ───────────────────────────────────────────────────────────────

    pub fn get_reviews(&self, plugin_id: &str) -> Vec<PluginReview> {
        self.reviews
            .get(plugin_id)
            .map(|r| r.clone())
            .unwrap_or_default()
    }

    pub fn add_review(&self, review: PluginReview) {
        self.reviews
            .entry(review.plugin_id.clone())
            .or_default()
            .push(review);
    }

    // ── Publishers ────────────────────────────────────────────────────────────

    pub fn get_publisher(&self, publisher_id: &str) -> Option<PublisherInfo> {
        self.publishers.get(publisher_id).map(|p| p.clone())
    }

    pub fn upsert_publisher(&self, publisher: PublisherInfo) {
        self.publishers.insert(publisher.id.clone(), publisher);
    }

    // ── Analytics ─────────────────────────────────────────────────────────────

    pub fn get_analytics(&self, plugin_id: &str) -> Option<PluginAnalytics> {
        self.analytics.get(plugin_id).map(|a| a.clone())
    }

    pub fn upsert_analytics(&self, analytics: PluginAnalytics) {
        self.analytics.insert(analytics.plugin_id.clone(), analytics);
    }
}

impl Default for MarketplaceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(id: &str) -> MarketplaceEntry {
        MarketplaceEntry {
            plugin_id: id.to_string(),
            name: format!("Plugin {id}"),
            version: "1.0.0".to_string(),
            description: Some("A test plugin.".to_string()),
            download_url: format!("https://marketplace.draox-server.io/plugins/{id}/1.0.0.dxp"),
            checksum_sha256: "abc123".to_string(),
        }
    }

    #[test]
    fn test_upsert_and_get() {
        let reg = MarketplaceRegistry::new();
        reg.upsert(sample_entry("io.draox.clans"));

        let entry = reg.get_entry("io.draox.clans").expect("entry should exist");
        assert_eq!(entry.plugin_id, "io.draox.clans");
        assert_eq!(entry.version, "1.0.0");
    }

    #[test]
    fn test_upsert_overwrites() {
        let reg = MarketplaceRegistry::new();
        reg.upsert(sample_entry("io.draox.clans"));

        let mut updated = sample_entry("io.draox.clans");
        updated.version = "2.0.0".to_string();
        reg.upsert(updated);

        assert_eq!(reg.get_entry("io.draox.clans").unwrap().version, "2.0.0");
    }

    #[test]
    fn test_remove() {
        let reg = MarketplaceRegistry::new();
        reg.upsert(sample_entry("io.draox.messaging"));

        assert!(reg.remove("io.draox.messaging"));
        assert!(!reg.remove("io.draox.messaging")); // already gone
        assert!(reg.get_entry("io.draox.messaging").is_none());
    }

    #[test]
    fn test_all_and_len() {
        let reg = MarketplaceRegistry::new();
        assert!(reg.is_empty());

        reg.upsert(sample_entry("io.draox.a"));
        reg.upsert(sample_entry("io.draox.b"));
        reg.upsert(sample_entry("io.draox.c"));

        assert_eq!(reg.len(), 3);
        assert_eq!(reg.all().len(), 3);
        assert!(!reg.is_empty());
    }
}
