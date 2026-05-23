use server_config::model::SqlConfig;
use server_core::Result;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::{MySqlPool, Row};
use std::time::Duration;
use tracing::info;

use crate::backend::{BoxFuture, StorageBackend};
use crate::error::into_storage_error;

/// MySQL/MariaDB-backed key-value storage.
///
/// Stores JSON values in a `kv_store` table keyed by `(namespace, key)`.
/// Compatible with both MySQL 8+ and MariaDB 10.5+.
pub struct MySqlStorage {
    pool: MySqlPool,
}

impl MySqlStorage {
    /// Connect to a MySQL/MariaDB database using the provided SQL config.
    ///
    /// # Example URL formats
    /// - `mysql://user:pass@localhost:3306/draox`
    /// - `mariadb://user:pass@localhost:3306/draox`
    pub async fn new(config: &SqlConfig) -> Result<Self> {
        let pool = MySqlPoolOptions::new()
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

        info!("MySQL storage initialized: {}", config.url);
        Ok(storage)
    }

    /// Create the `kv_store` table if it does not already exist.
    ///
    /// Note: `key` is a reserved word in MySQL — backtick-quoted in DDL.
    /// Uses `VARCHAR(255)` for indexed columns (MySQL index length limit).
    async fn run_migrations(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS kv_store (
                namespace  VARCHAR(255) NOT NULL,
                `key`      VARCHAR(255) NOT NULL,
                value      LONGTEXT     NOT NULL,
                updated_at VARCHAR(64)  NOT NULL,
                PRIMARY KEY (namespace, `key`)
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(into_storage_error)?;

        Ok(())
    }
}

impl StorageBackend for MySqlStorage {
    fn get(&self, namespace: &str, key: &str) -> BoxFuture<'_, Result<Option<serde_json::Value>>> {
        let namespace = namespace.to_owned();
        let key = key.to_owned();

        Box::pin(async move {
            let row =
                sqlx::query("SELECT value FROM kv_store WHERE namespace = ? AND `key` = ?")
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
                r#"INSERT INTO kv_store (namespace, `key`, value, updated_at)
                   VALUES (?, ?, ?, ?)
                   ON DUPLICATE KEY UPDATE value = VALUES(value), updated_at = VALUES(updated_at)"#,
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
                sqlx::query("DELETE FROM kv_store WHERE namespace = ? AND `key` = ?")
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
                "SELECT `key` FROM kv_store WHERE namespace = ? AND `key` LIKE ?",
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

    /// Helper: skip test if MYSQL_TEST_URL is not set.
    fn test_url() -> Option<String> {
        std::env::var("MYSQL_TEST_URL").ok()
    }

    async fn make_storage() -> Option<MySqlStorage> {
        let url = test_url()?;
        let config = SqlConfig {
            url,
            max_connections: 2,
            min_connections: 1,
            idle_timeout_secs: 60,
            max_lifetime_secs: 300,
            run_migrations: true,
        };
        MySqlStorage::new(&config).await.ok()
    }

    /// Clean up test data in a specific namespace.
    async fn cleanup(storage: &MySqlStorage, namespace: &str) {
        let _ = sqlx::query("DELETE FROM kv_store WHERE namespace = ?")
            .bind(namespace)
            .execute(&storage.pool)
            .await;
    }

    #[tokio::test]
    #[ignore] // Requires: MYSQL_TEST_URL=mysql://user:pass@localhost:3306/test_db
    async fn test_mysql_set_and_get() {
        let store = make_storage().await.expect("MySQL not available");
        cleanup(&store, "my_test").await;

        let val = json!({"name": "draox", "version": 1});
        store.set("my_test", "key1", val.clone()).await.unwrap();

        let result = store.get("my_test", "key1").await.unwrap();
        assert_eq!(result, Some(val));

        cleanup(&store, "my_test").await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_mysql_get_missing() {
        let store = make_storage().await.expect("MySQL not available");

        let result = store.get("my_test", "nonexistent").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    #[ignore]
    async fn test_mysql_set_overwrite() {
        let store = make_storage().await.expect("MySQL not available");
        cleanup(&store, "my_test").await;

        store.set("my_test", "key", json!(1)).await.unwrap();
        store.set("my_test", "key", json!(2)).await.unwrap();

        let result = store.get("my_test", "key").await.unwrap();
        assert_eq!(result, Some(json!(2)));

        cleanup(&store, "my_test").await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_mysql_delete() {
        let store = make_storage().await.expect("MySQL not available");
        cleanup(&store, "my_test").await;

        store.set("my_test", "key", json!("hello")).await.unwrap();
        let deleted = store.delete("my_test", "key").await.unwrap();
        assert!(deleted);

        let result = store.get("my_test", "key").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    #[ignore]
    async fn test_mysql_delete_missing() {
        let store = make_storage().await.expect("MySQL not available");

        let deleted = store.delete("my_test", "nope").await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    #[ignore]
    async fn test_mysql_list_keys() {
        let store = make_storage().await.expect("MySQL not available");
        cleanup(&store, "my_test").await;

        store.set("my_test", "user:1", json!(1)).await.unwrap();
        store.set("my_test", "user:2", json!(2)).await.unwrap();
        store.set("my_test", "session:1", json!(3)).await.unwrap();

        let mut keys = store.list_keys("my_test", "user:").await.unwrap();
        keys.sort();
        assert_eq!(keys, vec!["user:1", "user:2"]);

        cleanup(&store, "my_test").await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_mysql_namespace_isolation() {
        let store = make_storage().await.expect("MySQL not available");
        cleanup(&store, "my_ns_a").await;
        cleanup(&store, "my_ns_b").await;

        store.set("my_ns_a", "shared_key", json!("alpha")).await.unwrap();
        store.set("my_ns_b", "shared_key", json!("beta")).await.unwrap();

        let a = store.get("my_ns_a", "shared_key").await.unwrap();
        let b = store.get("my_ns_b", "shared_key").await.unwrap();

        assert_eq!(a, Some(json!("alpha")));
        assert_eq!(b, Some(json!("beta")));

        cleanup(&store, "my_ns_a").await;
        cleanup(&store, "my_ns_b").await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_mysql_json_values() {
        let store = make_storage().await.expect("MySQL not available");
        cleanup(&store, "my_test").await;

        let complex = json!({
            "users": [
                {"id": 1, "name": "Alice", "active": true},
                {"id": 2, "name": "Bob", "active": false}
            ],
            "metadata": {"version": "1.0", "count": 2}
        });

        store.set("my_test", "complex", complex.clone()).await.unwrap();
        let result = store.get("my_test", "complex").await.unwrap();
        assert_eq!(result, Some(complex));

        cleanup(&store, "my_test").await;
    }
}
