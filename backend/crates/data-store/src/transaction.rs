//! Transaction support for batching and atomically applying storage operations.
//!
//! Provides a [`Transaction`] that accumulates [`TransactionOp`] operations
//! (set and delete), and [`execute_transaction`] which applies them all against
//! a [`StorageBackend`]. On error, any operations applied so far are rolled back
//! by deleting any keys that were successfully set.

use server_core::Result;

use crate::backend::StorageBackend;

// ── Operation ────────────────────────────────────────────────────────────────

/// A single operation recorded inside a [`Transaction`].
pub enum TransactionOp {
    /// Write a JSON value to `(namespace, key)`.
    Set {
        namespace: String,
        key: String,
        value: serde_json::Value,
    },
    /// Delete the value at `(namespace, key)`.
    Delete { namespace: String, key: String },
}

// ── Transaction ───────────────────────────────────────────────────────────────

/// Accumulates storage operations and applies them atomically via
/// [`execute_transaction`].
///
/// # Example
///
/// ```rust,ignore
/// let mut tx = Transaction::new();
/// tx.set("sessions", "s1", serde_json::json!({"state": "active"}));
/// tx.delete("sessions", "old_key");
/// execute_transaction(&backend, tx).await?;
/// ```
pub struct Transaction {
    operations: Vec<TransactionOp>,
    committed: bool,
}

impl Transaction {
    /// Create an empty transaction.
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            committed: false,
        }
    }

    /// Queue a set operation for `(namespace, key) = value`.
    pub fn set(
        &mut self,
        namespace: impl Into<String>,
        key: impl Into<String>,
        value: serde_json::Value,
    ) {
        self.operations.push(TransactionOp::Set {
            namespace: namespace.into(),
            key: key.into(),
            value,
        });
    }

    /// Queue a delete operation for `(namespace, key)`.
    pub fn delete(&mut self, namespace: impl Into<String>, key: impl Into<String>) {
        self.operations.push(TransactionOp::Delete {
            namespace: namespace.into(),
            key: key.into(),
        });
    }

    /// Return a reference to the queued operations (for inspection / testing).
    pub fn operations(&self) -> &[TransactionOp] {
        &self.operations
    }

    /// Number of operations queued.
    pub fn len(&self) -> usize {
        self.operations.len()
    }

    /// `true` if no operations have been queued.
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    /// Whether this transaction has already been committed.
    pub fn is_committed(&self) -> bool {
        self.committed
    }
}

impl Default for Transaction {
    fn default() -> Self {
        Self::new()
    }
}

// ── execute_transaction ───────────────────────────────────────────────────────

/// Execute a [`Transaction`] against a [`StorageBackend`].
///
/// Operations are applied in order. If any operation fails, all **set**
/// operations applied so far are rolled back by deleting the affected keys.
/// Delete operations that were already applied cannot be reliably undone, so
/// the caller should treat a rollback error as a partial-failure condition.
///
/// Returns `Ok(())` if every operation succeeded.
pub async fn execute_transaction(
    backend: &dyn StorageBackend,
    mut tx: Transaction,
) -> Result<()> {
    // Track which keys were set successfully so we can roll them back on failure.
    let mut applied_sets: Vec<(String, String)> = Vec::new();

    for op in tx.operations.drain(..) {
        match op {
            TransactionOp::Set {
                namespace,
                key,
                value,
            } => {
                if let Err(e) = backend.set(&namespace, &key, value).await {
                    // Roll back previously applied sets.
                    rollback(backend, &applied_sets).await;
                    return Err(e);
                }
                applied_sets.push((namespace, key));
            }
            TransactionOp::Delete { namespace, key } => {
                if let Err(e) = backend.delete(&namespace, &key).await {
                    rollback(backend, &applied_sets).await;
                    return Err(e);
                }
            }
        }
    }

    tx.committed = true;
    Ok(())
}

/// Best-effort rollback: delete every key that was set during the transaction.
async fn rollback(backend: &dyn StorageBackend, applied_sets: &[(String, String)]) {
    for (namespace, key) in applied_sets {
        // Ignore individual rollback errors — we are already in an error path.
        let _ = backend.delete(namespace, key).await;
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite::SqliteStorage;
    use serde_json::json;

    async fn make_store() -> SqliteStorage {
        SqliteStorage::new_in_memory().await.unwrap()
    }

    #[tokio::test]
    async fn test_transaction_empty_succeeds() {
        let store = make_store().await;
        let tx = Transaction::new();
        assert!(tx.is_empty());
        let result = execute_transaction(&store, tx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_transaction_set_operations_applied() {
        let store = make_store().await;
        let mut tx = Transaction::new();
        tx.set("ns", "k1", json!("value1"));
        tx.set("ns", "k2", json!(42));
        assert_eq!(tx.len(), 2);

        execute_transaction(&store, tx).await.unwrap();

        let v1 = store.get("ns", "k1").await.unwrap();
        let v2 = store.get("ns", "k2").await.unwrap();
        assert_eq!(v1, Some(json!("value1")));
        assert_eq!(v2, Some(json!(42)));
    }

    #[tokio::test]
    async fn test_transaction_delete_operation() {
        let store = make_store().await;

        // Pre-populate a key.
        store.set("ns", "to_delete", json!("bye")).await.unwrap();

        let mut tx = Transaction::new();
        tx.delete("ns", "to_delete");
        execute_transaction(&store, tx).await.unwrap();

        let result = store.get("ns", "to_delete").await.unwrap();
        assert_eq!(result, None, "key should be deleted after transaction");
    }

    #[tokio::test]
    async fn test_transaction_mixed_set_and_delete() {
        let store = make_store().await;

        store.set("ns", "old", json!("gone")).await.unwrap();

        let mut tx = Transaction::new();
        tx.set("ns", "new_key", json!({"created": true}));
        tx.delete("ns", "old");

        execute_transaction(&store, tx).await.unwrap();

        assert_eq!(store.get("ns", "new_key").await.unwrap(), Some(json!({"created": true})));
        assert_eq!(store.get("ns", "old").await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_transaction_op_introspection() {
        let mut tx = Transaction::new();
        tx.set("a", "k", json!(1));
        tx.delete("a", "x");
        assert_eq!(tx.len(), 2);
        assert!(!tx.is_empty());

        let ops = tx.operations();
        assert_eq!(ops.len(), 2);

        // Verify op types via pattern matching.
        assert!(matches!(&ops[0], TransactionOp::Set { key, .. } if key == "k"));
        assert!(matches!(&ops[1], TransactionOp::Delete { key, .. } if key == "x"));
    }
}
