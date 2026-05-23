pub mod app;
pub mod audit;
pub mod auth;
pub mod billing;
pub mod cache;
pub mod config;
pub mod connections;
pub mod dynamic_routes;
pub mod guard;
pub mod marketplace;
pub mod metrics;
pub mod plugins;
pub mod sessions;
pub mod users;
pub mod ws_streams;

use crate::state::AppState;
use axum::routing::{delete, get, post, put};
use axum::Router;

/// Build the complete admin API router.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        // Auth
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/me", get(auth::me))
        // Users
        .route("/api/users", get(users::list_users).post(users::create_user))
        .route(
            "/api/users/{username}",
            put(users::update_user).delete(users::delete_user),
        )
        // Health & info
        .route("/api/health", get(app::health))
        .route("/api/health/detailed", get(app::health_detailed))
        .route("/api/info", get(app::info))
        // Connections (static path before parameterized)
        .route("/api/connections", get(connections::list_connections))
        .route("/api/connections/stats", get(connections::connection_stats))
        .route(
            "/api/connections/{id}",
            get(connections::get_connection).delete(connections::disconnect_connection),
        )
        // Sessions (static path before parameterized)
        .route("/api/sessions", get(sessions::list_sessions))
        .route(
            "/api/sessions/{id}",
            get(sessions::get_session).delete(sessions::destroy_session),
        )
        .route(
            "/api/sessions/{id}/drain",
            post(sessions::drain_session),
        )
        .route(
            "/api/sessions/{id}/metrics",
            get(sessions::session_metrics),
        )
        // Plugins
        .route("/api/plugins", get(plugins::list_plugins))
        .route("/api/plugins/{id}", get(plugins::get_plugin))
        .route(
            "/api/plugins/{id}/activate",
            post(plugins::activate_plugin),
        )
        .route(
            "/api/plugins/{id}/deactivate",
            post(plugins::deactivate_plugin),
        )
        .route("/api/plugins/{id}/enable", post(plugins::enable_plugin))
        .route("/api/plugins/{id}/disable", post(plugins::disable_plugin))
        .route(
            "/api/plugins/{id}/restart",
            post(plugins::restart_plugin),
        )
        .route(
            "/api/plugins/{id}/health",
            get(plugins::plugin_health),
        )
        // Traffic guard (static paths before parameterized)
        .route("/api/guard/stats", get(guard::guard_stats))
        .route("/api/guard/ban", post(guard::ban_ip))
        .route("/api/guard/unban", post(guard::unban_ip))
        .route("/api/guard/bans", get(guard::list_bans))
        .route("/api/guard/whitelist", post(guard::add_whitelist))
        .route("/api/guard/blacklist", post(guard::add_blacklist))
        .route(
            "/api/guard/reputation/{ip}",
            get(guard::get_reputation),
        )
        // Config
        .route("/api/config", get(config::get_config))
        .route("/api/config/reload", post(config::reload_config))
        // Billing (static paths before parameterized)
        .route("/api/billing/plans", get(billing::list_plans))
        .route(
            "/api/billing/usage/{client_id}",
            get(billing::get_usage),
        )
        .route(
            "/api/billing/plan/{client_id}",
            put(billing::set_plan),
        )
        // Cache
        .route("/api/cache/stats", get(cache::cache_stats))
        .route("/api/cache/health", get(cache::cache_health))
        .route("/api/cache/flush", post(cache::flush_cache))
        // Audit
        .route("/api/audit", get(audit::list_audit))
        .route("/api/audit/{id}", get(audit::get_audit_entry))
        // Metrics
        .route("/api/metrics", get(metrics::get_metrics))
        .route(
            "/api/metrics/prometheus",
            get(metrics::get_metrics_prometheus),
        )
        .route(
            "/api/metrics/activity",
            get(metrics::activity_summary),
        )
        // Marketplace (static paths before parameterized)
        .route(
            "/api/marketplace/search",
            get(marketplace::search_plugins),
        )
        .route(
            "/api/marketplace/featured",
            get(marketplace::list_featured),
        )
        .route(
            "/api/marketplace/popular",
            get(marketplace::list_popular),
        )
        .route(
            "/api/marketplace/categories",
            get(marketplace::list_categories),
        )
        .route(
            "/api/marketplace/publish",
            post(marketplace::publish_plugin),
        )
        .route(
            "/api/marketplace/plugins/{id}",
            get(marketplace::get_marketplace_plugin),
        )
        .route(
            "/api/marketplace/plugins/{id}/versions",
            get(marketplace::get_plugin_versions),
        )
        .route(
            "/api/marketplace/plugins/{id}/reviews",
            get(marketplace::get_plugin_reviews).post(marketplace::add_plugin_review),
        )
        .route(
            "/api/marketplace/plugins/{id}/analytics",
            get(marketplace::get_plugin_analytics),
        )
        .route(
            "/api/marketplace/publishers/{id}",
            get(marketplace::get_publisher),
        )
        // Dynamic plugin routes (static paths before parameterized)
        .route("/api/routes", get(dynamic_routes::list_routes))
        .route(
            "/api/routes/{plugin_id}",
            get(dynamic_routes::get_plugin_routes)
                .delete(dynamic_routes::unregister_plugin_routes),
        )
        .route(
            "/api/routes/{plugin_id}/register",
            post(dynamic_routes::register_route),
        )
        // WebSocket streams
        .route("/ws/events", get(ws_streams::ws_events))
        .route("/ws/connections", get(ws_streams::ws_connections))
        .route("/ws/plugins", get(ws_streams::ws_plugins))
        .route("/ws/guard", get(ws_streams::ws_guard))
        .route("/ws/metrics", get(ws_streams::ws_metrics))
        .with_state(state)
}
