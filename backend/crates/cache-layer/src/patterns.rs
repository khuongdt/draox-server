//! Common cache usage patterns: cache-aside, read-through, and write-through.
//!
//! These patterns sit on top of the [`CacheBackend`] trait and encode
//! well-known strategies for keeping a cache consistent with an upstream
//! data source.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use server_core::Result;

use crate::backend::{BoxFuture, CacheBackend};

// ── DataLoader / DataWriter traits ────────────────────────────────────────────

/// Source-of-truth loader used by read-through and write-through caches.
///
/// Implementations fetch a raw byte value from a backing store (database,
/// remote service, etc.).
pub trait DataLoader: Send + Sync {
    /// Load the value for `key` from the backing source.
    ///
    /// Returns `Ok(Some(bytes))` when found, `Ok(None)` for a cache miss, or
    /// `Err` on I/O / query failure.
    fn load(&self, key: &str) -> BoxFuture<'_, Result<Option<Vec<u8>>>>;
}

/// Backing-store writer used by the write-through cache.
pub trait DataWriter: Send + Sync {
    /// Persist `value` under `key` in the backing store.
    fn store(&self, key: &str, value: Vec<u8>) -> BoxFuture<'_, Result<()>>;
}

// ── cache_aside ───────────────────────────────────────────────────────────────

/// **Cache-aside** pattern.
///
/// 1. Check the cache for `key`.
/// 2. On hit → return the cached bytes.
/// 3. On miss → call `loader` to fetch the value.
/// 4. If the loader returns `Some(bytes)`, store them in the cache (with
///    optional TTL) and return the bytes.
///
/// The caller is responsible for keeping the cache and backing store in sync
/// for writes; this function only handles reads.
pub async fn cache_aside<F, Fut>(
    cache: &dyn CacheBackend,
    key: &str,
    loader: F,
    ttl: Option<Duration>,
) -> Result<Option<Vec<u8>>>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<Option<Vec<u8>>>>,
{
    // 1. Cache hit?
    if let Some(cached) = cache.get(key).await? {
        return Ok(Some(cached));
    }

    // 2. Cache miss — load from backing store.
    let loaded = loader().await?;

    // 3. Populate the cache if the loader found a value.
    if let Some(ref bytes) = loaded {
        let ttl_secs = ttl.map(|d| d.as_secs());
        cache.set(key, bytes.clone(), ttl_secs).await?;
    }

    Ok(loaded)
}

// ── ReadThroughCache ──────────────────────────────────────────────────────────

/// **Read-through** cache.
///
/// The cache is the single point of contact for reads. On a miss it
/// automatically fetches from the [`DataLoader`] and populates the cache
/// before returning the value, so callers never deal with the loader directly.
pub struct ReadThroughCache {
    cache: Arc<dyn CacheBackend>,
    loader: Arc<dyn DataLoader>,
}

impl ReadThroughCache {
    /// Create a read-through cache backed by `cache`, loading missing values
    /// from `loader`.
    pub fn new(cache: Arc<dyn CacheBackend>, loader: Arc<dyn DataLoader>) -> Self {
        Self { cache, loader }
    }

    /// Get a value, auto-populating the cache on miss.
    ///
    /// `ttl` is forwarded to the backend's `set` call; pass `None` to use the
    /// backend's default TTL.
    pub async fn get(&self, key: &str, ttl: Option<Duration>) -> Result<Option<Vec<u8>>> {
        // 1. Cache hit?
        if let Some(cached) = self.cache.get(key).await? {
            return Ok(Some(cached));
        }

        // 2. Miss — ask the loader.
        let loaded = self.loader.load(key).await?;

        // 3. Populate cache on success.
        if let Some(ref bytes) = loaded {
            let ttl_secs = ttl.map(|d| d.as_secs());
            self.cache.set(key, bytes.clone(), ttl_secs).await?;
        }

        Ok(loaded)
    }
}

// ── WriteThroughCache ─────────────────────────────────────────────────────────

/// **Write-through** cache.
///
/// Every write is applied to **both** the cache and the backing store
/// simultaneously (cache first, then store). If the backing store write fails,
/// the cached value is deleted so stale data is not served.
pub struct WriteThroughCache {
    cache: Arc<dyn CacheBackend>,
    store: Arc<dyn DataWriter>,
}

impl WriteThroughCache {
    /// Create a write-through cache.
    pub fn new(cache: Arc<dyn CacheBackend>, store: Arc<dyn DataWriter>) -> Self {
        Self { cache, store }
    }

    /// Write `value` to both cache and backing store.
    ///
    /// On backing-store failure the cache entry is invalidated to avoid serving
    /// stale data, and the original error is returned.
    pub async fn set(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> Result<()> {
        let ttl_secs = ttl.map(|d| d.as_secs());

        // 1. Write to cache.
        self.cache.set(key, value.clone(), ttl_secs).await?;

        // 2. Write to backing store; on failure evict the cached value.
        if let Err(e) = self.store.store(key, value).await {
            let _ = self.cache.delete(key).await;
            return Err(e);
        }

        Ok(())
    }

    /// Read a value from the cache only (does not consult the backing store).
    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.cache.get(key).await
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryCache;
    use server_config::model::MemoryCacheConfig;
    use server_core::Error;

    fn make_cache() -> Arc<MemoryCache> {
        Arc::new(MemoryCache::new(&MemoryCacheConfig {
            max_capacity: 100,
            ttl_secs: 300,
        }))
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    struct StaticLoader {
        value: Option<Vec<u8>>,
    }

    impl DataLoader for StaticLoader {
        fn load(&self, _key: &str) -> BoxFuture<'_, Result<Option<Vec<u8>>>> {
            let value = self.value.clone();
            Box::pin(async move { Ok(value) })
        }
    }

    struct FailingLoader;

    impl DataLoader for FailingLoader {
        fn load(&self, _key: &str) -> BoxFuture<'_, Result<Option<Vec<u8>>>> {
            Box::pin(async { Err(Error::Cache("loader failure".into())) })
        }
    }

    struct MemoryWriter {
        inner: std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>,
    }

    impl MemoryWriter {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                inner: std::sync::Mutex::new(std::collections::HashMap::new()),
            })
        }

        fn get(&self, key: &str) -> Option<Vec<u8>> {
            self.inner.lock().unwrap().get(key).cloned()
        }
    }

    impl DataWriter for MemoryWriter {
        fn store(&self, key: &str, value: Vec<u8>) -> BoxFuture<'_, Result<()>> {
            let key = key.to_string();
            Box::pin(async move {
                self.inner.lock().unwrap().insert(key, value);
                Ok(())
            })
        }
    }

    // ── cache_aside tests ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_cache_aside_miss_populates_cache() {
        let cache = make_cache();
        let result = cache_aside(
            cache.as_ref(),
            "key1",
            || async { Ok(Some(b"loaded".to_vec())) },
            Some(Duration::from_secs(60)),
        )
        .await
        .unwrap();

        assert_eq!(result, Some(b"loaded".to_vec()));

        // Subsequent get should come from cache without calling loader again.
        let cached = cache.get("key1").await.unwrap();
        assert_eq!(cached, Some(b"loaded".to_vec()));
    }

    #[tokio::test]
    async fn test_cache_aside_hit_skips_loader() {
        let cache = make_cache();
        cache.set("hit_key", b"cached_value".to_vec(), None).await.unwrap();

        let loader_called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let loader_called_clone = loader_called.clone();

        let result = cache_aside(
            cache.as_ref(),
            "hit_key",
            || async move {
                loader_called_clone.store(true, std::sync::atomic::Ordering::Relaxed);
                Ok(Some(b"from_loader".to_vec()))
            },
            None,
        )
        .await
        .unwrap();

        assert_eq!(result, Some(b"cached_value".to_vec()));
        assert!(!loader_called.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[tokio::test]
    async fn test_cache_aside_loader_returns_none() {
        let cache = make_cache();
        let result = cache_aside(
            cache.as_ref(),
            "missing",
            || async { Ok(None) },
            None,
        )
        .await
        .unwrap();

        assert_eq!(result, None);
        // Nothing should have been stored in cache.
        assert!(!cache.exists("missing").await.unwrap());
    }

    // ── ReadThroughCache tests ────────────────────────────────────────────────

    #[tokio::test]
    async fn test_read_through_populates_on_miss() {
        let cache = make_cache();
        let loader = Arc::new(StaticLoader {
            value: Some(b"db_value".to_vec()),
        });
        let rt = ReadThroughCache::new(cache.clone(), loader);

        let result = rt.get("rtkey", None).await.unwrap();
        assert_eq!(result, Some(b"db_value".to_vec()));

        // Should now be in cache.
        assert!(cache.exists("rtkey").await.unwrap());
    }

    #[tokio::test]
    async fn test_read_through_loader_error_propagated() {
        let cache = make_cache();
        let loader = Arc::new(FailingLoader);
        let rt = ReadThroughCache::new(cache, loader);

        let result = rt.get("key", None).await;
        assert!(result.is_err());
    }

    // ── WriteThroughCache tests ───────────────────────────────────────────────

    #[tokio::test]
    async fn test_write_through_writes_cache_and_store() {
        let cache = make_cache();
        let writer = MemoryWriter::new();
        let wt = WriteThroughCache::new(cache.clone(), writer.clone());

        wt.set("wt_key", b"wt_value".to_vec(), None).await.unwrap();

        let from_cache = cache.get("wt_key").await.unwrap();
        assert_eq!(from_cache, Some(b"wt_value".to_vec()));

        let from_store = writer.get("wt_key");
        assert_eq!(from_store, Some(b"wt_value".to_vec()));
    }

    #[tokio::test]
    async fn test_write_through_get_reads_cache() {
        let cache = make_cache();
        let writer = MemoryWriter::new();
        let wt = WriteThroughCache::new(cache.clone(), writer);

        cache.set("existing", b"data".to_vec(), None).await.unwrap();
        let result = wt.get("existing").await.unwrap();
        assert_eq!(result, Some(b"data".to_vec()));
    }
}
