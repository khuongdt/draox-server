//! Cache layer for the Draox Server.
//!
//! Provides a [`CacheBackend`] trait and concrete implementations:
//!
//! - [`MemoryCache`] — high-performance in-memory cache powered by
//!   [moka](https://github.com/moka-rs/moka) (always available, no external
//!   dependencies).
//! - [`RedisCache`] — Redis-backed cache powered by
//!   [fred](https://github.com/aembke/fred.rs) (requires a running Redis instance).
//!
//! Use [`create_cache_backend`] to build the appropriate backend from config.

pub mod backend;
pub mod error;
pub mod keys;
pub mod memory;
pub mod patterns;
pub mod redis;
pub mod serialization;

pub use backend::{BoxFuture, CacheBackend};
pub use error::cache_error;
pub use keys::CacheKeys;
pub use memory::MemoryCache;
pub use patterns::{DataLoader, DataWriter, ReadThroughCache, WriteThroughCache, cache_aside};
pub use redis::RedisCache;
pub use serialization::{
    BincodeSerializer, CacheSerializer, JsonSerializer, MessagePackSerializer,
    SerializationFormat,
};

use server_config::model::CacheConfig;
use std::sync::Arc;
use tracing::{info, warn};

/// Create the appropriate [`CacheBackend`] based on configuration.
///
/// - If `config.redis.enabled` is `true`, attempts to connect to Redis.
///   On connection failure, falls back to [`MemoryCache`] with a warning.
/// - Otherwise, creates a [`MemoryCache`] directly.
///
/// Returns `(backend, backend_name)` where `backend_name` is `"redis"` or `"memory"`.
pub async fn create_cache_backend(config: &CacheConfig) -> (Arc<dyn CacheBackend>, &'static str) {
    if config.redis.enabled {
        match RedisCache::connect(&config.redis).await {
            Ok(redis_cache) => {
                info!(
                    url = %config.redis.url,
                    pool_size = config.redis.pool_size,
                    ttl_secs = config.redis.default_ttl_secs,
                    "cache backend: Redis"
                );
                return (Arc::new(redis_cache) as Arc<dyn CacheBackend>, "redis");
            }
            Err(e) => {
                warn!(
                    error = %e,
                    "Redis connection failed, falling back to memory cache"
                );
            }
        }
    }

    let memory = MemoryCache::new(&config.memory);
    info!(
        max_capacity = config.memory.max_capacity,
        ttl_secs = config.memory.ttl_secs,
        "cache backend: Memory"
    );
    (Arc::new(memory), "memory")
}

#[cfg(test)]
mod tests {
    use super::*;
    use server_config::model::MemoryCacheConfig;

    /// Helper: build a [`MemoryCache`] with the given capacity and TTL.
    fn make_cache(max_capacity: u64, ttl_secs: u64) -> MemoryCache {
        let config = MemoryCacheConfig {
            max_capacity,
            ttl_secs,
        };
        MemoryCache::new(&config)
    }

    #[tokio::test]
    async fn test_memory_cache_set_get() {
        let cache = make_cache(100, 300);
        cache.set("key1", b"hello".to_vec(), None).await.unwrap();

        let result = cache.get("key1").await.unwrap();
        assert_eq!(result, Some(b"hello".to_vec()));
    }

    #[tokio::test]
    async fn test_memory_cache_get_missing() {
        let cache = make_cache(100, 300);

        let result = cache.get("nonexistent").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_memory_cache_delete() {
        let cache = make_cache(100, 300);
        cache.set("to_delete", b"value".to_vec(), None).await.unwrap();

        let deleted = cache.delete("to_delete").await.unwrap();
        assert!(deleted);

        // Key should be gone after deletion.
        let result = cache.get("to_delete").await.unwrap();
        assert_eq!(result, None);

        // Deleting a non-existent key returns false.
        let deleted_again = cache.delete("to_delete").await.unwrap();
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_memory_cache_exists() {
        let cache = make_cache(100, 300);
        assert!(!cache.exists("key").await.unwrap());

        cache.set("key", b"data".to_vec(), None).await.unwrap();
        assert!(cache.exists("key").await.unwrap());
    }

    #[tokio::test]
    async fn test_memory_cache_overwrite() {
        let cache = make_cache(100, 300);
        cache.set("key", b"first".to_vec(), None).await.unwrap();
        cache.set("key", b"second".to_vec(), None).await.unwrap();

        let result = cache.get("key").await.unwrap();
        assert_eq!(result, Some(b"second".to_vec()));
    }

    #[tokio::test]
    async fn test_memory_cache_ttl_expiry() {
        // Use a very short TTL (1 second) and wait for expiration.
        let cache = make_cache(100, 1);
        cache.set("ephemeral", b"gone_soon".to_vec(), None).await.unwrap();

        // Value should be present immediately.
        assert!(cache.get("ephemeral").await.unwrap().is_some());

        // Wait for the TTL to expire.
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // Value should have been evicted.
        let result = cache.get("ephemeral").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_memory_cache_default_config() {
        let config = MemoryCacheConfig::default();
        assert_eq!(config.max_capacity, 10_000);
        assert_eq!(config.ttl_secs, 300);

        let cache = MemoryCache::new(&config);
        assert_eq!(cache.default_ttl_secs(), 300);
        assert_eq!(cache.entry_count(), 0);
    }

    #[tokio::test]
    async fn test_memory_backend_name() {
        let cache = make_cache(100, 300);
        assert_eq!(cache.backend_name(), "memory");
    }

    #[tokio::test]
    async fn test_memory_flush() {
        let cache = make_cache(100, 300);
        cache.set("a", b"1".to_vec(), None).await.unwrap();
        cache.set("b", b"2".to_vec(), None).await.unwrap();

        // Verify keys exist before flush
        assert!(cache.exists("a").await.unwrap());
        assert!(cache.exists("b").await.unwrap());

        cache.flush().await.unwrap();

        // Keys should be gone after flush
        assert!(!cache.exists("a").await.unwrap());
        assert!(!cache.exists("b").await.unwrap());
    }

    #[tokio::test]
    async fn test_memory_entry_count_async() {
        let cache = make_cache(100, 300);
        cache.set("x", b"1".to_vec(), None).await.unwrap();
        cache.set("y", b"2".to_vec(), None).await.unwrap();

        // entry_count_async returns an approximate count (moka is eventually
        // consistent). Verify entries exist via get instead.
        assert!(cache.get("x").await.unwrap().is_some());
        assert!(cache.get("y").await.unwrap().is_some());

        // The async entry count should still succeed without error.
        let count = cache.entry_count_async().await.unwrap();
        assert!(count <= 2, "count should not exceed 2, got {count}");
    }

    #[tokio::test]
    async fn test_create_cache_backend_memory() {
        let config = CacheConfig::default(); // redis.enabled = false
        let (backend, name) = create_cache_backend(&config).await;
        assert_eq!(name, "memory");
        assert_eq!(backend.backend_name(), "memory");
    }

    #[tokio::test]
    async fn test_create_cache_backend_redis_fallback() {
        // Redis enabled but unreachable → should fallback to memory
        let mut config = CacheConfig::default();
        config.redis.enabled = true;
        config.redis.url = "redis://localhost:59999".to_string();

        let (backend, name) = create_cache_backend(&config).await;
        assert_eq!(name, "memory");
        assert_eq!(backend.backend_name(), "memory");
    }
}
