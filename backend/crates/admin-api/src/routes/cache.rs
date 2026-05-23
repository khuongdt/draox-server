use crate::response::ApiResponse;
use crate::state::AppState;
use axum::extract::State;
use axum::response::IntoResponse;
use serde::Serialize;
use std::time::Instant;

#[derive(Serialize)]
pub struct CacheStatsResponse {
    pub backend: String,
    pub entries: u64,
}

#[derive(Serialize)]
pub struct CacheHealthResponse {
    pub backend: String,
    pub healthy: bool,
    pub latency_ms: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct CacheFlushResponse {
    pub flushed: bool,
}

/// GET /api/cache/stats — cache backend statistics.
pub async fn cache_stats(State(state): State<AppState>) -> impl IntoResponse {
    let entries = state.cache.entry_count_async().await.unwrap_or(0);

    ApiResponse::ok(CacheStatsResponse {
        backend: state.cache.backend_name().to_string(),
        entries,
    })
}

/// GET /api/cache/health — cache backend health check with latency.
pub async fn cache_health(State(state): State<AppState>) -> impl IntoResponse {
    let start = Instant::now();
    let result = state.cache.health_check().await;
    let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

    let (healthy, error) = match result {
        Ok(()) => (true, None),
        Err(e) => (false, Some(e.to_string())),
    };

    ApiResponse::ok(CacheHealthResponse {
        backend: state.cache.backend_name().to_string(),
        healthy,
        latency_ms,
        error,
    })
}

/// POST /api/cache/flush — flush all cache entries.
pub async fn flush_cache(State(state): State<AppState>) -> impl IntoResponse {
    let flushed = state.cache.flush().await.is_ok();
    ApiResponse::ok(CacheFlushResponse { flushed })
}
