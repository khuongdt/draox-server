use crate::error::ApiError;
use crate::response::ApiResponse;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::Json;
use plugin_host::{
    MarketplacePlugin, NewReview, PluginCategory, PluginReview, PluginVersion, SearchQuery, SortBy,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Search ────────────────────────────────────────────────────────────────────

/// GET /api/marketplace/search?q=...&category=...&sort=...&page=...&page_size=...
pub async fn search_plugins(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let q = SearchQuery {
        query: params.get("q").cloned(),
        category: params.get("category").and_then(|c| parse_category(c)),
        tags: params
            .get("tags")
            .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default(),
        sort_by: params
            .get("sort")
            .map(|s| parse_sort(s))
            .unwrap_or(SortBy::Relevance),
        page: params
            .get("page")
            .and_then(|p| p.parse().ok())
            .unwrap_or(1),
        page_size: params
            .get("page_size")
            .and_then(|p| p.parse().ok())
            .unwrap_or(20),
    };
    let result = state.marketplace.search(&q);
    ApiResponse::ok(result)
}

// ── Single plugin ─────────────────────────────────────────────────────────────

/// GET /api/marketplace/plugins/{id}
pub async fn get_marketplace_plugin(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let plugin = state
        .marketplace
        .get_plugin(&id)
        .ok_or_else(|| ApiError::not_found(format!("marketplace plugin '{id}' not found")))?;
    Ok(ApiResponse::ok(plugin))
}

// ── Versions ──────────────────────────────────────────────────────────────────

/// GET /api/marketplace/plugins/{id}/versions
pub async fn get_plugin_versions(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let versions = state.marketplace.get_versions(&id);
    ApiResponse::ok(VersionListResponse {
        plugin_id: id,
        total: versions.len(),
        versions,
    })
}

#[derive(Serialize)]
pub struct VersionListResponse {
    pub plugin_id: String,
    pub total: usize,
    pub versions: Vec<PluginVersion>,
}

// ── Reviews ───────────────────────────────────────────────────────────────────

/// GET /api/marketplace/plugins/{id}/reviews
pub async fn get_plugin_reviews(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let reviews = state.marketplace.get_reviews(&id);
    ApiResponse::ok(ReviewListResponse {
        plugin_id: id,
        total: reviews.len(),
        reviews,
    })
}

#[derive(Serialize)]
pub struct ReviewListResponse {
    pub plugin_id: String,
    pub total: usize,
    pub reviews: Vec<PluginReview>,
}

/// POST /api/marketplace/plugins/{id}/reviews
pub async fn add_plugin_review(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<NewReview>,
) -> Result<impl IntoResponse, ApiError> {
    if req.rating < 1 || req.rating > 5 {
        return Err(ApiError::bad_request("rating must be between 1 and 5"));
    }
    // Verify the plugin exists before accepting the review.
    if state.marketplace.get_plugin(&id).is_none() {
        return Err(ApiError::not_found(format!(
            "marketplace plugin '{id}' not found"
        )));
    }
    let review = PluginReview {
        id: uuid::Uuid::new_v4().to_string(),
        plugin_id: id.clone(),
        reviewer: req.author_name.clone(),
        rating: req.rating,
        comment: req.body.unwrap_or_default(),
        created_at: chrono::Utc::now(),
    };
    state
        .marketplace
        .add_review(review)
        .map_err(|e| ApiError::bad_request(e))?;
    Ok(ApiResponse::<()>::message(format!(
        "review submitted for plugin '{id}'"
    )))
}

// ── Featured / Popular / Categories ──────────────────────────────────────────

/// GET /api/marketplace/featured
pub async fn list_featured(State(state): State<AppState>) -> impl IntoResponse {
    let plugins = state.marketplace.list_featured();
    ApiResponse::ok(PluginListResponse {
        total: plugins.len(),
        plugins,
    })
}

/// GET /api/marketplace/popular
pub async fn list_popular(State(state): State<AppState>) -> impl IntoResponse {
    let plugins = state.marketplace.list_popular(20);
    ApiResponse::ok(PluginListResponse {
        total: plugins.len(),
        plugins,
    })
}

#[derive(Serialize)]
pub struct PluginListResponse {
    pub total: usize,
    pub plugins: Vec<MarketplacePlugin>,
}

/// GET /api/marketplace/categories
pub async fn list_categories() -> impl IntoResponse {
    let categories: Vec<String> = PluginCategory::all()
        .into_iter()
        .map(|c| c.to_string())
        .collect();
    ApiResponse::ok(serde_json::json!({ "categories": categories }))
}

// ── Analytics ─────────────────────────────────────────────────────────────────

/// GET /api/marketplace/plugins/{id}/analytics
pub async fn get_plugin_analytics(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let analytics = state
        .marketplace
        .get_analytics(&id)
        .ok_or_else(|| ApiError::not_found(format!("analytics for plugin '{id}' not found")))?;
    Ok(ApiResponse::ok(analytics))
}

// ── Publish ───────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct PublishRequest {
    pub plugin: MarketplacePlugin,
}

/// POST /api/marketplace/publish
pub async fn publish_plugin(
    State(state): State<AppState>,
    Json(req): Json<PublishRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let id = req.plugin.id.clone();
    state
        .marketplace
        .publish(req.plugin)
        .map_err(|e| ApiError::bad_request(e))?;
    Ok(ApiResponse::<()>::message(format!(
        "plugin '{id}' submitted to marketplace"
    )))
}

// ── Publishers ────────────────────────────────────────────────────────────────

/// GET /api/marketplace/publishers/{id}
pub async fn get_publisher(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let publisher = state
        .marketplace
        .get_publisher(&id)
        .ok_or_else(|| ApiError::not_found(format!("publisher '{id}' not found")))?;
    Ok(ApiResponse::ok(publisher))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_category(s: &str) -> Option<PluginCategory> {
    match s.to_lowercase().as_str() {
        "gameplay" => Some(PluginCategory::Gameplay),
        "communication" => Some(PluginCategory::Communication),
        "security" => Some(PluginCategory::Security),
        "analytics" => Some(PluginCategory::Analytics),
        "integration" => Some(PluginCategory::Integration),
        "utility" => Some(PluginCategory::Utility),
        "other" => Some(PluginCategory::Other),
        _ => None,
    }
}

fn parse_sort(s: &str) -> SortBy {
    match s.to_lowercase().as_str() {
        "downloads" => SortBy::Downloads,
        "rating" => SortBy::Rating,
        "newest" => SortBy::Newest,
        "updated" => SortBy::Updated,
        _ => SortBy::Relevance,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::auth::JwtConfig;
    use crate::auth_store::AdminUserStore;
    use crate::routes::build_router;
    use crate::state::AppState;
    use activity_log::metrics::MetricsCollector;
    use activity_log::{ActivityLog, AuditLog};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use billing::UsageTracker;
    use connection_manager::SessionManager;
    use plugin_host::{ContextBuilder, FullMarketplaceRegistry, PluginRegistry, RouteRegistry};
    use server_config::model::{SessionConfig, TrafficGuardConfig};
    use server_core::event::EventBus;
    use server_core::{ConnectionId, Error, ServerInfo};
    use socket_server::handler::{BoxFuture, ConnectionHandler};
    use socket_server::tracker::ConnectionTracker;
    use std::sync::Arc;
    use tower::ServiceExt;
    use traffic_guard::TrafficGuard;

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

    async fn make_state() -> AppState {
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
        let cache: Arc<dyn cache_layer::CacheBackend> = Arc::new(
            cache_layer::MemoryCache::new(&server_config::model::MemoryCacheConfig::default()),
        );
        let storage: Arc<dyn data_store::StorageBackend> = Arc::new(
            data_store::SqliteStorage::new_in_memory().await.unwrap(),
        );
        let auth_store = Arc::new(AdminUserStore::new(Arc::clone(&storage)));
        let ctx_builder = ContextBuilder::new(ServerInfo::default(), Arc::clone(&event_bus), Arc::clone(&cache));
        let plugin_registry = Arc::new(PluginRegistry::new(ctx_builder, Arc::clone(&event_bus)));
        let activity_log = Arc::new(ActivityLog::new(10000));
        let audit_log = Arc::new(AuditLog::new(10000));
        let metrics = Arc::new(MetricsCollector::new());
        let usage_tracker = Arc::new(UsageTracker::new());
        let marketplace = Arc::new(FullMarketplaceRegistry::new());
        let route_registry = Arc::new(RouteRegistry::new());
        let config = server_config::DraoxConfig::default();
        AppState {
            connection_tracker: tracker,
            session_manager: session_mgr,
            traffic_guard: guard,
            plugin_registry,
            activity_log,
            metrics,
            usage_tracker,
            audit_log,
            event_bus,
            marketplace,
            route_registry,
            cache,
            storage,
            jwt_config: JwtConfig::default(),
            auth_store,
            config: Arc::new(std::sync::RwLock::new(config)),
            config_path: String::new(),
        }
    }

    #[tokio::test]
    async fn test_search_empty_returns_ok() {
        let state = make_state().await;
        let app = build_router(state).await;

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/marketplace/search")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["total"], 0);
    }

    #[tokio::test]
    async fn test_featured_empty() {
        let state = make_state().await;
        let app = build_router(state).await;

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/marketplace/featured")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"]["total"], 0);
    }

    #[tokio::test]
    async fn test_get_plugin_not_found() {
        let state = make_state().await;
        let app = build_router(state).await;

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/marketplace/plugins/io.draox.nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_categories_returns_list() {
        let state = make_state().await;
        let app = build_router(state).await;

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/marketplace/categories")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let cats = &json["data"]["categories"];
        assert!(cats.is_array());
        assert!(!cats.as_array().unwrap().is_empty());
    }
}
