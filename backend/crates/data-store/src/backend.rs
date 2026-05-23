use server_core::Result;
use std::future::Future;
use std::pin::Pin;

/// Type alias for a boxed, pinned, Send future — used for trait methods returning async results.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Storage backend abstraction for key-value operations organized by namespace.
///
/// All keys and values are scoped under a namespace, allowing multiple subsystems
/// (plugins, sessions, etc.) to share the same storage without key collisions.
pub trait StorageBackend: Send + Sync + 'static {
    /// Retrieve a JSON value by namespace and key.
    /// Returns `None` if the key does not exist.
    fn get(&self, namespace: &str, key: &str) -> BoxFuture<'_, Result<Option<serde_json::Value>>>;

    /// Store a JSON value under the given namespace and key.
    /// Overwrites any existing value for the same (namespace, key) pair.
    fn set(&self, namespace: &str, key: &str, value: serde_json::Value) -> BoxFuture<'_, Result<()>>;

    /// Delete a value by namespace and key.
    /// Returns `true` if a row was actually deleted, `false` if the key did not exist.
    fn delete(&self, namespace: &str, key: &str) -> BoxFuture<'_, Result<bool>>;

    /// List all keys in a namespace whose names start with the given prefix.
    /// Pass an empty string to list all keys in the namespace.
    fn list_keys(&self, namespace: &str, prefix: &str) -> BoxFuture<'_, Result<Vec<String>>>;
}
