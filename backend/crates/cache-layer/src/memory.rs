//! In-memory cache backend powered by [moka](https://github.com/moka-rs/moka).
//!
//! Uses `moka::future::Cache` for async-safe, concurrent, LRU-based caching
//! with configurable capacity and TTL.

use moka::future::Cache;
use server_config::model::MemoryCacheConfig;
use std::time::Duration;
use tracing::debug;

use crate::backend::{BoxFuture, CacheBackend};

/// In-memory cache backed by moka.
///
/// Entries are evicted based on a global time-to-live (TTL) and an LRU policy
/// once `max_capacity` is reached.
pub struct MemoryCache {
    cache: Cache<String, Vec<u8>>,
    default_ttl_secs: u64,
}

impl MemoryCache {
    /// Create a new [`MemoryCache`] from the given configuration.
    ///
    /// - `config.max_capacity` — maximum number of entries the cache can hold.
    /// - `config.ttl_secs` — default time-to-live for every entry (in seconds).
    pub fn new(config: &MemoryCacheConfig) -> Self {
        let cache = Cache::builder()
            .max_capacity(config.max_capacity)
            .time_to_live(Duration::from_secs(config.ttl_secs))
            .build();

        debug!(
            max_capacity = config.max_capacity,
            ttl_secs = config.ttl_secs,
            "memory cache initialized"
        );

        Self {
            cache,
            default_ttl_secs: config.ttl_secs,
        }
    }

    /// Return the default TTL configured for this cache (in seconds).
    pub fn default_ttl_secs(&self) -> u64 {
        self.default_ttl_secs
    }

    /// Return the approximate number of entries currently in the cache.
    pub fn entry_count(&self) -> u64 {
        self.cache.entry_count()
    }
}

impl CacheBackend for MemoryCache {
    fn get(&self, key: &str) -> BoxFuture<'_, server_core::Result<Option<Vec<u8>>>> {
        let key = key.to_string();
        Box::pin(async move {
            let value = self.cache.get(&key).await;
            Ok(value)
        })
    }

    fn set(
        &self,
        key: &str,
        value: Vec<u8>,
        _ttl_secs: Option<u64>,
    ) -> BoxFuture<'_, server_core::Result<()>> {
        // moka's global TTL is set at cache creation time. Per-entry TTL is not
        // supported by the basic builder API, so we honour the global default.
        // The `_ttl_secs` parameter is accepted for API compatibility with
        // backends that support per-key TTL (e.g. Redis).
        let key = key.to_string();
        Box::pin(async move {
            self.cache.insert(key, value).await;
            Ok(())
        })
    }

    fn delete(&self, key: &str) -> BoxFuture<'_, server_core::Result<bool>> {
        let key = key.to_string();
        Box::pin(async move {
            // Check existence first so we can report whether the key was present.
            let existed = self.cache.get(&key).await.is_some();
            self.cache.invalidate(&key).await;
            Ok(existed)
        })
    }

    fn exists(&self, key: &str) -> BoxFuture<'_, server_core::Result<bool>> {
        let key = key.to_string();
        Box::pin(async move {
            let found = self.cache.get(&key).await.is_some();
            Ok(found)
        })
    }

    // ── Optional methods ─────────────────────────────────────────────────────

    fn backend_name(&self) -> &str {
        "memory"
    }

    fn entry_count_async(&self) -> BoxFuture<'_, server_core::Result<u64>> {
        Box::pin(async { Ok(self.cache.entry_count()) })
    }

    fn health_check(&self) -> BoxFuture<'_, server_core::Result<()>> {
        // In-memory cache is always healthy.
        Box::pin(async { Ok(()) })
    }

    fn flush(&self) -> BoxFuture<'_, server_core::Result<()>> {
        Box::pin(async {
            self.cache.invalidate_all();
            // moka's invalidate_all is lazy; run_pending_tasks forces eviction.
            self.cache.run_pending_tasks().await;
            Ok(())
        })
    }
}
