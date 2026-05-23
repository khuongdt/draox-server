//! Cache backend trait defining the interface for all cache implementations.

use server_core::Result;
use std::future::Future;
use std::pin::Pin;

/// A boxed future that is `Send`-safe, used for trait object return types.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Trait that all cache backends must implement.
///
/// Provides basic key-value cache operations: get, set, delete, and exists.
/// All operations are async (returning [`BoxFuture`]) and work with raw bytes.
///
/// Optional methods (`backend_name`, `entry_count`, `health_check`, `flush`)
/// have default implementations so existing backends are not broken.
pub trait CacheBackend: Send + Sync + 'static {
    /// Retrieve a value by key.
    ///
    /// Returns `Ok(Some(bytes))` if the key exists, `Ok(None)` if it does not.
    fn get(&self, key: &str) -> BoxFuture<'_, Result<Option<Vec<u8>>>>;

    /// Store a value with an optional TTL in seconds.
    ///
    /// If `ttl_secs` is `None`, the backend's default TTL is used.
    fn set(&self, key: &str, value: Vec<u8>, ttl_secs: Option<u64>) -> BoxFuture<'_, Result<()>>;

    /// Delete a key from the cache.
    ///
    /// Returns `Ok(true)` if the key was present before deletion, `Ok(false)` otherwise.
    fn delete(&self, key: &str) -> BoxFuture<'_, Result<bool>>;

    /// Check whether a key exists in the cache.
    fn exists(&self, key: &str) -> BoxFuture<'_, Result<bool>>;

    // ── Optional methods ─────────────────────────────────────────────────────

    /// Human-readable name of the backend (e.g. `"memory"`, `"redis"`).
    fn backend_name(&self) -> &str {
        "unknown"
    }

    /// Approximate number of entries currently held.
    ///
    /// Not all backends can report this cheaply; the default returns `0`.
    fn entry_count_async(&self) -> BoxFuture<'_, Result<u64>> {
        Box::pin(async { Ok(0) })
    }

    /// Lightweight health check (e.g. Redis `PING`).
    ///
    /// Returns `Ok(())` when the backend is reachable and functioning.
    fn health_check(&self) -> BoxFuture<'_, Result<()>> {
        Box::pin(async { Ok(()) })
    }

    /// Flush (invalidate) all entries in the cache.
    fn flush(&self) -> BoxFuture<'_, Result<()>> {
        Box::pin(async { Ok(()) })
    }
}
