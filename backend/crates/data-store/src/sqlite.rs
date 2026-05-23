use server_core::Result;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Row, SqlitePool};
use std::time::Duration;
use tracing::info;

use crate::backend::{BoxFuture, StorageBackend};
use crate::error::into_storage_error;

/// SQLite-backed key-value storage.
///
/// Stores JSON values in a `kv_store` table keyed by `(namespace, key)`.
/// Suitable for development, testing, and single-node deployments.
pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    /// Connect to an existing SQLite database at the given URL and run migrations.
    ///
    /// # Example URL formats
    /// - `sqlite:data.db` — relative file path
    /// - `sqlite:///absolute/path/data.db` — absolute file path
    pub async fn new(url: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await
            .map_err(into_storage_error)?;

        let storage = Self { pool };
        storage.run_migrations().await?;

        info!("SQLite storage initialized: {url}");
        Ok(storage)
    }

    /// Connect to a SQLite database using the provided SQL config.
    ///
    /// Uses pool settings from config (max/min connections, timeouts).
    pub async fn from_config(config: &server_config::model::SqlConfig) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .idle_timeout(Duration::from_secs(config.idle_timeout_secs))
            .max_lifetime(Duration::from_secs(config.max_lifetime_secs))
            .connect(&config.url)
            .await
            .map_err(into_storage_error)?;

        let storage = Self { pool };

        if config.run_migrations {
            storage.run_migrations().await?;
        }

        info!("SQLite storage initialized (from config): {}", config.url);
        Ok(storage)
    }

    /// Create an in-memory SQLite database — ideal for unit tests.
    ///
    /// Each call produces an isolated database that lives as long as the pool is open.
    pub async fn new_in_memory() -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .map_err(into_storage_error)?;

        let storage = Self { pool };
        storage.run_migrations().await?;

        info!("SQLite in-memory storage initialized");
        Ok(storage)
    }

    /// Create the `kv_store` table if it does not already exist.
    async fn run_migrations(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS kv_store (
                namespace TEXT NOT NULL,
                key       TEXT NOT NULL,
                value     TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (namespace, key)
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(into_storage_error)?;

        Ok(())
    }
}

impl StorageBackend for SqliteStorage {
    fn get(&self, namespace: &str, key: &str) -> BoxFuture<'_, Result<Option<serde_json::Value>>> {
        let namespace = namespace.to_owned();
        let key = key.to_owned();

        Box::pin(async move {
            let row = sqlx::query("SELECT value FROM kv_store WHERE namespace = ? AND key = ?")
                .bind(&namespace)
                .bind(&key)
                .fetch_optional(&self.pool)
                .await
                .map_err(into_storage_error)?;

            match row {
                Some(row) => {
                    let raw: String = row.get("value");
                    let value: serde_json::Value = serde_json::from_str(&raw)?;
                    Ok(Some(value))
                }
                None => Ok(None),
            }
        })
    }

    fn set(
        &self,
        namespace: &str,
        key: &str,
        value: serde_json::Value,
    ) -> BoxFuture<'_, Result<()>> {
        let namespace = namespace.to_owned();
        let key = key.to_owned();

        Box::pin(async move {
            let json_text = serde_json::to_string(&value)?;
            let now = chrono::Utc::now().to_rfc3339();

            sqlx::query(
                "INSERT OR REPLACE INTO kv_store (namespace, key, value, updated_at) VALUES (?, ?, ?, ?)",
            )
            .bind(&namespace)
            .bind(&key)
            .bind(&json_text)
            .bind(&now)
            .execute(&self.pool)
            .await
            .map_err(into_storage_error)?;

            Ok(())
        })
    }

    fn delete(&self, namespace: &str, key: &str) -> BoxFuture<'_, Result<bool>> {
        let namespace = namespace.to_owned();
        let key = key.to_owned();

        Box::pin(async move {
            let result =
                sqlx::query("DELETE FROM kv_store WHERE namespace = ? AND key = ?")
                    .bind(&namespace)
                    .bind(&key)
                    .execute(&self.pool)
                    .await
                    .map_err(into_storage_error)?;

            Ok(result.rows_affected() > 0)
        })
    }

    fn list_keys(&self, namespace: &str, prefix: &str) -> BoxFuture<'_, Result<Vec<String>>> {
        let namespace = namespace.to_owned();
        let pattern = format!("{prefix}%");

        Box::pin(async move {
            let rows = sqlx::query(
                "SELECT key FROM kv_store WHERE namespace = ? AND key LIKE ?",
            )
            .bind(&namespace)
            .bind(&pattern)
            .fetch_all(&self.pool)
            .await
            .map_err(into_storage_error)?;

            let keys: Vec<String> = rows.iter().map(|r| r.get("key")).collect();
            Ok(keys)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_new_in_memory() {
        let storage = SqliteStorage::new_in_memory().await;
        assert!(storage.is_ok(), "should create in-memory storage successfully");
    }

    #[tokio::test]
    async fn test_set_and_get() {
        let store = SqliteStorage::new_in_memory().await.unwrap();
        let val = json!({"name": "draox", "version": 1});

        store.set("test", "key1", val.clone()).await.unwrap();

        let result = store.get("test", "key1").await.unwrap();
        assert_eq!(result, Some(val));
    }

    #[tokio::test]
    async fn test_get_missing() {
        let store = SqliteStorage::new_in_memory().await.unwrap();

        let result = store.get("ns", "nonexistent").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_set_overwrite() {
        let store = SqliteStorage::new_in_memory().await.unwrap();

        store.set("ns", "key", json!(1)).await.unwrap();
        store.set("ns", "key", json!(2)).await.unwrap();

        let result = store.get("ns", "key").await.unwrap();
        assert_eq!(result, Some(json!(2)));
    }

    #[tokio::test]
    async fn test_delete() {
        let store = SqliteStorage::new_in_memory().await.unwrap();

        store.set("ns", "key", json!("hello")).await.unwrap();
        let deleted = store.delete("ns", "key").await.unwrap();

        assert!(deleted, "delete should return true for existing key");

        let result = store.get("ns", "key").await.unwrap();
        assert_eq!(result, None, "key should be gone after delete");
    }

    #[tokio::test]
    async fn test_delete_missing() {
        let store = SqliteStorage::new_in_memory().await.unwrap();

        let deleted = store.delete("ns", "nope").await.unwrap();
        assert!(!deleted, "delete should return false for non-existent key");
    }

    #[tokio::test]
    async fn test_list_keys() {
        let store = SqliteStorage::new_in_memory().await.unwrap();

        store.set("app", "user:1", json!(1)).await.unwrap();
        store.set("app", "user:2", json!(2)).await.unwrap();
        store.set("app", "session:1", json!(3)).await.unwrap();

        let mut keys = store.list_keys("app", "user:").await.unwrap();
        keys.sort();

        assert_eq!(keys, vec!["user:1", "user:2"]);
    }

    #[tokio::test]
    async fn test_list_keys_empty() {
        let store = SqliteStorage::new_in_memory().await.unwrap();

        let keys = store.list_keys("empty_ns", "").await.unwrap();
        assert!(keys.is_empty(), "empty namespace should yield no keys");
    }

    #[tokio::test]
    async fn test_namespace_isolation() {
        let store = SqliteStorage::new_in_memory().await.unwrap();

        store.set("ns_a", "shared_key", json!("alpha")).await.unwrap();
        store.set("ns_b", "shared_key", json!("beta")).await.unwrap();

        let a = store.get("ns_a", "shared_key").await.unwrap();
        let b = store.get("ns_b", "shared_key").await.unwrap();

        assert_eq!(a, Some(json!("alpha")));
        assert_eq!(b, Some(json!("beta")));
    }

    #[tokio::test]
    async fn test_json_values() {
        let store = SqliteStorage::new_in_memory().await.unwrap();

        let complex = json!({
            "users": [
                {"id": 1, "name": "Alice", "active": true},
                {"id": 2, "name": "Bob", "active": false}
            ],
            "metadata": {
                "version": "1.0",
                "count": 2
            }
        });

        store.set("data", "complex", complex.clone()).await.unwrap();

        let result = store.get("data", "complex").await.unwrap();
        assert_eq!(result, Some(complex));
    }
}
