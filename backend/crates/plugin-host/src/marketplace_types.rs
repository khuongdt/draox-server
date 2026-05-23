use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Full metadata for a plugin listed in the marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplacePlugin {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: PublisherInfo,
    pub category: PluginCategory,
    pub tags: Vec<String>,
    pub downloads: u64,
    pub active_installs: u64,
    pub rating: f64,
    pub rating_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_paid: bool,
    pub price_cents: Option<u64>,
    pub featured: bool,
    /// SemVer requirement for server compatibility, e.g. ">=1.0.0".
    pub compatibility: String,
}

/// Information about a plugin publisher.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublisherInfo {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub verified: bool,
    pub joined_at: DateTime<Utc>,
}

/// Top-level category for a marketplace plugin.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PluginCategory {
    Gameplay,
    Communication,
    Security,
    Analytics,
    Integration,
    Utility,
    Other,
}

impl std::fmt::Display for PluginCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            PluginCategory::Gameplay => "Gameplay",
            PluginCategory::Communication => "Communication",
            PluginCategory::Security => "Security",
            PluginCategory::Analytics => "Analytics",
            PluginCategory::Integration => "Integration",
            PluginCategory::Utility => "Utility",
            PluginCategory::Other => "Other",
        };
        write!(f, "{s}")
    }
}

impl PluginCategory {
    /// All known categories (used for the `/api/marketplace/categories` endpoint).
    pub fn all() -> Vec<PluginCategory> {
        vec![
            PluginCategory::Gameplay,
            PluginCategory::Communication,
            PluginCategory::Security,
            PluginCategory::Analytics,
            PluginCategory::Integration,
            PluginCategory::Utility,
            PluginCategory::Other,
        ]
    }
}

/// Request body for submitting a new plugin review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewReview {
    pub author_name: String,
    /// Rating from 1 to 5.
    pub rating: u8,
    pub title: Option<String>,
    pub body: Option<String>,
}

/// Thin HTTP client stub for calling the remote marketplace registry API.
///
/// In production this would make HTTP requests to `marketplace.draox-server.io`.
/// Actual sync is handled by a background task; this struct carries configuration.
#[derive(Debug, Clone)]
pub struct MarketplaceClient {
    pub base_url: String,
}

impl MarketplaceClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
        }
    }
}

impl Default for MarketplaceClient {
    fn default() -> Self {
        Self::new("https://marketplace.draox-server.io")
    }
}

/// A specific release of a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginVersion {
    pub version: String,
    pub release_notes: String,
    pub min_server_version: String,
    pub published_at: DateTime<Utc>,
    pub download_url: String,
    pub checksum: String,
    pub size_bytes: u64,
    pub dependencies: Vec<PluginDependency>,
}

/// A dependency on another marketplace plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    pub plugin_id: String,
    pub version_requirement: String,
}

/// A user review for a marketplace plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginReview {
    pub id: String,
    pub plugin_id: String,
    pub reviewer: String,
    /// Rating in the range 1–5.
    pub rating: u8,
    pub comment: String,
    pub created_at: DateTime<Utc>,
}

/// A query for searching the marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: Option<String>,
    pub category: Option<PluginCategory>,
    pub tags: Vec<String>,
    pub sort_by: SortBy,
    pub page: u32,
    pub page_size: u32,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            query: None,
            category: None,
            tags: vec![],
            sort_by: SortBy::Relevance,
            page: 1,
            page_size: 20,
        }
    }
}

/// Criteria used to order marketplace search results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortBy {
    Relevance,
    Downloads,
    Rating,
    Newest,
    Updated,
}

/// Paginated search result from the marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub plugins: Vec<MarketplacePlugin>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

/// Download / install analytics for a single plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAnalytics {
    pub plugin_id: String,
    pub total_downloads: u64,
    pub active_installs: u64,
    /// Daily download counts, keyed by ISO-8601 date string (e.g. "2025-01-15").
    pub daily_downloads: Vec<(String, u64)>,
    /// Install count per version string.
    pub version_distribution: HashMap<String, u64>,
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_publisher() -> PublisherInfo {
        PublisherInfo {
            id: "pub-1".to_string(),
            name: "Acme Corp".to_string(),
            email: Some("hello@acme.io".to_string()),
            verified: true,
            joined_at: Utc::now(),
        }
    }

    fn sample_plugin() -> MarketplacePlugin {
        MarketplacePlugin {
            id: "io.draox.sample".to_string(),
            name: "Sample Plugin".to_string(),
            description: "A sample marketplace plugin".to_string(),
            version: "1.0.0".to_string(),
            author: sample_publisher(),
            category: PluginCategory::Utility,
            tags: vec!["utility".to_string(), "sample".to_string()],
            downloads: 1_000,
            active_installs: 500,
            rating: 4.5,
            rating_count: 42,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            is_paid: false,
            price_cents: None,
            featured: false,
            compatibility: ">=1.0.0".to_string(),
        }
    }

    #[test]
    fn test_marketplace_plugin_serialization_roundtrip() {
        let plugin = sample_plugin();
        let json = serde_json::to_string(&plugin).expect("serialize");
        let restored: MarketplacePlugin = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.id, plugin.id);
        assert_eq!(restored.name, plugin.name);
        assert_eq!(restored.downloads, plugin.downloads);
        assert_eq!(restored.rating_count, plugin.rating_count);
    }

    #[test]
    fn test_default_search_query() {
        let q = SearchQuery::default();
        assert!(q.query.is_none());
        assert!(q.category.is_none());
        assert!(q.tags.is_empty());
        assert_eq!(q.page, 1);
        assert_eq!(q.page_size, 20);
    }

    #[test]
    fn test_plugin_version_serialization_roundtrip() {
        let v = PluginVersion {
            version: "2.3.1".to_string(),
            release_notes: "Bug fixes and improvements".to_string(),
            min_server_version: "1.0.0".to_string(),
            published_at: Utc::now(),
            download_url: "https://marketplace.draox-server.io/dl/plugin-2.3.1.dxp".to_string(),
            checksum: "abc123".to_string(),
            size_bytes: 204_800,
            dependencies: vec![PluginDependency {
                plugin_id: "io.draox.core-utils".to_string(),
                version_requirement: ">=1.0.0".to_string(),
            }],
        };
        let json = serde_json::to_string(&v).expect("serialize");
        let restored: PluginVersion = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.version, v.version);
        assert_eq!(restored.size_bytes, v.size_bytes);
        assert_eq!(restored.dependencies.len(), 1);
    }

    #[test]
    fn test_plugin_analytics_serialization_roundtrip() {
        let analytics = PluginAnalytics {
            plugin_id: "io.draox.sample".to_string(),
            total_downloads: 10_000,
            active_installs: 3_500,
            daily_downloads: vec![
                ("2025-01-01".to_string(), 120),
                ("2025-01-02".to_string(), 95),
            ],
            version_distribution: {
                let mut m = HashMap::new();
                m.insert("1.0.0".to_string(), 2000u64);
                m.insert("1.1.0".to_string(), 1500u64);
                m
            },
        };
        let json = serde_json::to_string(&analytics).expect("serialize");
        let restored: PluginAnalytics = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.total_downloads, analytics.total_downloads);
        assert_eq!(restored.daily_downloads.len(), 2);
        assert_eq!(restored.version_distribution.len(), 2);
    }

    #[test]
    fn test_plugin_category_display() {
        assert_eq!(PluginCategory::Gameplay.to_string(), "Gameplay");
        assert_eq!(PluginCategory::Communication.to_string(), "Communication");
        assert_eq!(PluginCategory::Security.to_string(), "Security");
        assert_eq!(PluginCategory::Other.to_string(), "Other");
    }

    #[test]
    fn test_paid_plugin() {
        let mut plugin = sample_plugin();
        plugin.is_paid = true;
        plugin.price_cents = Some(999);
        assert!(plugin.is_paid);
        assert_eq!(plugin.price_cents, Some(999));

        let json = serde_json::to_string(&plugin).expect("serialize");
        let restored: MarketplacePlugin = serde_json::from_str(&json).expect("deserialize");
        assert!(restored.is_paid);
        assert_eq!(restored.price_cents, Some(999));
    }
}
