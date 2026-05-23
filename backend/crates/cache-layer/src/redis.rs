//! Redis cache backend powered by [fred](https://github.com/aembke/fred.rs).
//!
//! Uses `fred::clients::Pool` for async-safe, connection-pooled Redis access.
//! Supports per-key TTL via Redis `SET ... EX`, unlike the in-memory backend
//! which only has a global TTL.

use fred::prelude::*;
use fred::interfaces::ServerInterface;
use server_config::model::RedisConfig;
use std::time::Duration;
use tracing::{debug, warn};

use crate::backend::{BoxFuture, CacheBackend};
use crate::error::cache_error;

/// Redis-backed cache using a `fred` connection pool.
///
/// All keys are stored as raw bytes via Redis byte-string values.
pub struct RedisCache {
    pool: Pool,
    default_ttl_secs: u64,
}

impl RedisCache {
    /// Connect to Redis using the given configuration.
    ///
    /// Returns an error if the connection cannot be established within 5 seconds.
    /// The caller should handle the error and fall back to [`MemoryCache`] if needed.
    pub async fn connect(config: &RedisConfig) -> server_core::Result<Self> {
        let redis_config =
            Config::from_url(&config.url).map_err(|e| cache_error(format!("invalid Redis URL: {e}")))?;

        let pool = Builder::from_config(redis_config)
            .with_connection_config(|conn| {
                conn.connection_timeout = Duration::from_secs(5);
            })
            .build_pool(config.pool_size as usize)
            .map_err(|e| cache_error(format!("failed to build Redis pool: {e}")))?;

        pool.init().await.map_err(|e| cache_error(format!("Redis connect failed: {e}")))?;

        // Verify the connection is alive.
        let _: String = pool
            .ping(None)
            .await
            .map_err(|e| cache_error(format!("Redis ping failed: {e}")))?;

        debug!(
            url = %config.url,
            pool_size = config.pool_size,
            default_ttl_secs = config.default_ttl_secs,
            "Redis cache connected"
        );

        Ok(Self {
            pool,
            default_ttl_secs: config.default_ttl_secs,
        })
    }

    /// Check whether the pool still has active connections.
    pub fn is_connected(&self) -> bool {
        self.pool.is_connected()
    }
}

impl CacheBackend for RedisCache {
    fn get(&self, key: &str) -> BoxFuture<'_, server_core::Result<Option<Vec<u8>>>> {
        let key = key.to_string();
        let pool = self.pool.clone();
        Box::pin(async move {
            let result: Option<Vec<u8>> = pool
                .get(&key)
                .await
                .map_err(|e| cache_error(format!("Redis GET failed: {e}")))?;
            Ok(result)
        })
    }

    fn set(
        &self,
        key: &str,
        value: Vec<u8>,
        ttl_secs: Option<u64>,
    ) -> BoxFuture<'_, server_core::Result<()>> {
        let key = key.to_string();
        let pool = self.pool.clone();
        let ttl = ttl_secs.unwrap_or(self.default_ttl_secs);
        Box::pin(async move {
            let expiration = if ttl > 0 {
                Some(Expiration::EX(ttl as i64))
            } else {
                None
            };
            pool.set::<(), _, _>(&key, value.as_slice(), expiration, None, false)
                .await
                .map_err(|e| cache_error(format!("Redis SET failed: {e}")))?;
            Ok(())
        })
    }

    fn delete(&self, key: &str) -> BoxFuture<'_, server_core::Result<bool>> {
        let key = key.to_string();
        let pool = self.pool.clone();
        Box::pin(async move {
            let removed: i64 = pool
                .del(&key)
                .await
                .map_err(|e| cache_error(format!("Redis DEL failed: {e}")))?;
            Ok(removed > 0)
        })
    }

    fn exists(&self, key: &str) -> BoxFuture<'_, server_core::Result<bool>> {
        let key = key.to_string();
        let pool = self.pool.clone();
        Box::pin(async move {
            let count: i64 = pool
                .exists(&key)
                .await
                .map_err(|e| cache_error(format!("Redis EXISTS failed: {e}")))?;
            Ok(count > 0)
        })
    }

    // ── Optional methods ─────────────────────────────────────────────────────

    fn backend_name(&self) -> &str {
        "redis"
    }

    fn entry_count_async(&self) -> BoxFuture<'_, server_core::Result<u64>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let size: i64 = pool
                .dbsize()
                .await
                .map_err(|e| cache_error(format!("Redis DBSIZE failed: {e}")))?;
            Ok(size as u64)
        })
    }

    fn health_check(&self) -> BoxFuture<'_, server_core::Result<()>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let _: String = pool
                .ping(None)
                .await
                .map_err(|e| cache_error(format!("Redis PING failed: {e}")))?;
            Ok(())
        })
    }

    fn flush(&self) -> BoxFuture<'_, server_core::Result<()>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            pool.flushall::<()>(false)
                .await
                .map_err(|e| cache_error(format!("Redis FLUSHALL failed: {e}")))?;
            Ok(())
        })
    }
}

impl Drop for RedisCache {
    fn drop(&mut self) {
        if self.pool.is_connected() {
            warn!("RedisCache dropped while still connected — call quit() for clean shutdown");
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────
//
// Redis tests require a running Redis instance. They are ignored by default.
// Run with: cargo test --package cache-layer -- --ignored

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> RedisConfig {
        RedisConfig {
            enabled: true,
            url: "redis://localhost:6379".to_string(),
            pool_size: 2,
            default_ttl_secs: 60,
        }
    }

    #[tokio::test]
    #[ignore = "requires running Redis instance"]
    async fn test_redis_connect_and_ping() {
        let cache = RedisCache::connect(&test_config()).await.unwrap();
        assert!(cache.is_connected());
        cache.health_check().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires running Redis instance"]
    async fn test_redis_set_get_delete() {
        let cache = RedisCache::connect(&test_config()).await.unwrap();

        // Clean up test key
        let _ = cache.delete("test:redis:key1").await;

        // SET
        cache
            .set("test:redis:key1", b"hello_redis".to_vec(), Some(30))
            .await
            .unwrap();

        // GET
        let val = cache.get("test:redis:key1").await.unwrap();
        assert_eq!(val, Some(b"hello_redis".to_vec()));

        // EXISTS
        assert!(cache.exists("test:redis:key1").await.unwrap());

        // DELETE
        let deleted = cache.delete("test:redis:key1").await.unwrap();
        assert!(deleted);

        // GET after delete
        let val = cache.get("test:redis:key1").await.unwrap();
        assert_eq!(val, None);

        // DELETE non-existent
        let deleted = cache.delete("test:redis:key1").await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    #[ignore = "requires running Redis instance"]
    async fn test_redis_backend_name() {
        let cache = RedisCache::connect(&test_config()).await.unwrap();
        assert_eq!(cache.backend_name(), "redis");
    }

    #[tokio::test]
    #[ignore = "requires running Redis instance"]
    async fn test_redis_entry_count() {
        let cache = RedisCache::connect(&test_config()).await.unwrap();
        // Just verify it returns a valid number (>= 0)
        let count = cache.entry_count_async().await.unwrap();
        assert!(count < u64::MAX);
    }

    #[tokio::test]
    async fn test_redis_connect_fails_gracefully() {
        // Connect to a non-existent Redis instance — should return an error, not panic.
        let config = RedisConfig {
            enabled: true,
            url: "redis://localhost:59999".to_string(),
            pool_size: 1,
            default_ttl_secs: 60,
        };
        let result = RedisCache::connect(&config).await;
        assert!(result.is_err());
    }
}
