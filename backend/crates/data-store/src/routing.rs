//! Read-replica routing for storage backends.
//!
//! [`ReadReplicaRouter`] wraps a primary [`StorageBackend`] and a list of
//! replica backends. Write operations (set / delete) always go to the primary.
//! Read operations (get / list_keys) are distributed across replicas in a
//! round-robin fashion; when no replicas are configured, reads fall through to
//! the primary.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use server_core::Result;

use crate::backend::{BoxFuture, StorageBackend};

// ── ReadReplicaRouter ─────────────────────────────────────────────────────────

/// Routes write operations to the primary backend and distributes reads across
/// replicas using round-robin selection.
///
/// Thread-safe: the replica counter uses [`AtomicUsize`] with relaxed ordering.
///
/// # Example
///
/// ```rust,ignore
/// let mut router = ReadReplicaRouter::new(primary);
/// router.add_replica(replica1);
/// router.add_replica(replica2);
///
/// // Reads go to replica1 or replica2 alternately.
/// let value = router.read_replica().get("ns", "key").await?;
/// // Writes always target the primary.
/// router.primary().set("ns", "key", json!(1)).await?;
/// ```
pub struct ReadReplicaRouter {
    primary: Arc<dyn StorageBackend>,
    replicas: Vec<Arc<dyn StorageBackend>>,
    next_replica: AtomicUsize,
}

impl ReadReplicaRouter {
    /// Create a router with only a primary — no replicas configured yet.
    pub fn new(primary: Arc<dyn StorageBackend>) -> Self {
        Self {
            primary,
            replicas: Vec::new(),
            next_replica: AtomicUsize::new(0),
        }
    }

    /// Add a read replica. Replicas are selected in the order they are added.
    pub fn add_replica(&mut self, replica: Arc<dyn StorageBackend>) {
        self.replicas.push(replica);
    }

    /// Return a reference to the primary backend for write operations.
    pub fn primary(&self) -> &dyn StorageBackend {
        self.primary.as_ref()
    }

    /// Return the next replica in round-robin order.
    ///
    /// Falls back to the primary when no replicas have been registered.
    pub fn read_replica(&self) -> &dyn StorageBackend {
        if self.replicas.is_empty() {
            return self.primary.as_ref();
        }
        let idx = self.next_replica.fetch_add(1, Ordering::Relaxed) % self.replicas.len();
        self.replicas[idx].as_ref()
    }

    /// Number of registered read replicas.
    pub fn replica_count(&self) -> usize {
        self.replicas.len()
    }
}

// Implement `StorageBackend` so the router itself can be used anywhere a
// backend is expected: reads go to replicas, writes go to the primary.
impl StorageBackend for ReadReplicaRouter {
    fn get(&self, namespace: &str, key: &str) -> BoxFuture<'_, Result<Option<serde_json::Value>>> {
        self.read_replica().get(namespace, key)
    }

    fn set(
        &self,
        namespace: &str,
        key: &str,
        value: serde_json::Value,
    ) -> BoxFuture<'_, Result<()>> {
        self.primary().set(namespace, key, value)
    }

    fn delete(&self, namespace: &str, key: &str) -> BoxFuture<'_, Result<bool>> {
        self.primary().delete(namespace, key)
    }

    fn list_keys(&self, namespace: &str, prefix: &str) -> BoxFuture<'_, Result<Vec<String>>> {
        self.read_replica().list_keys(namespace, prefix)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite::SqliteStorage;
    use serde_json::json;

    async fn make_backend() -> Arc<SqliteStorage> {
        Arc::new(SqliteStorage::new_in_memory().await.unwrap())
    }

    #[tokio::test]
    async fn test_router_no_replicas_falls_back_to_primary() {
        let primary = make_backend().await;
        let router = ReadReplicaRouter::new(primary.clone());

        assert_eq!(router.replica_count(), 0);

        // Reads and writes should both work via primary.
        router.primary().set("ns", "k", json!("v")).await.unwrap();
        let val = router.read_replica().get("ns", "k").await.unwrap();
        assert_eq!(val, Some(json!("v")));
    }

    #[tokio::test]
    async fn test_router_write_goes_to_primary() {
        let primary = make_backend().await;
        let replica = make_backend().await;

        let mut router = ReadReplicaRouter::new(primary.clone());
        router.add_replica(replica.clone());

        // Write via router → should appear on primary.
        router.set("ns", "key", json!(99)).await.unwrap();

        let on_primary = primary.get("ns", "key").await.unwrap();
        assert_eq!(on_primary, Some(json!(99)), "write should land on primary");

        let on_replica = replica.get("ns", "key").await.unwrap();
        assert_eq!(on_replica, None, "replica should not receive writes");
    }

    #[tokio::test]
    async fn test_router_round_robin_distribution() {
        let primary = make_backend().await;
        let replica_a = make_backend().await;
        let replica_b = make_backend().await;

        // Seed each replica with distinct values under the same key.
        replica_a.set("ns", "k", json!("from_a")).await.unwrap();
        replica_b.set("ns", "k", json!("from_b")).await.unwrap();

        let mut router = ReadReplicaRouter::new(primary.clone());
        router.add_replica(replica_a);
        router.add_replica(replica_b);

        assert_eq!(router.replica_count(), 2);

        // First read → replica_a, second read → replica_b.
        let first = router.read_replica().get("ns", "k").await.unwrap();
        let second = router.read_replica().get("ns", "k").await.unwrap();

        // The two reads should have hit different replicas.
        assert_ne!(first, second, "round-robin should alternate between replicas");
    }

    #[tokio::test]
    async fn test_router_delete_goes_to_primary() {
        let primary = make_backend().await;
        primary.set("ns", "del_me", json!(true)).await.unwrap();

        let router = ReadReplicaRouter::new(primary.clone());
        let deleted = router.delete("ns", "del_me").await.unwrap();

        assert!(deleted, "delete should return true for existing key");
        assert_eq!(primary.get("ns", "del_me").await.unwrap(), None);
    }
}
