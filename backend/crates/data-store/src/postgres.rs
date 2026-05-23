use server_config::model::SqlConfig;
use server_core::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use std::time::Duration;
use tracing::info;

use crate::backend::{BoxFuture, StorageBackend};
use crate::error::into_storage_error;

/// PostgreSQL-backed key-value storage.
///
/// Stores JSON values in a `kv_store` table keyed by `(namespace, key)`.
/// Suitable for production multi-node deployments requiring a shared database.
pub struct PostgresStorage {
    pool: PgPool,
}

impl PostgresStorage {
    /// Connect to a PostgreSQL database using the provided SQL config.
    ///
    /// # Example URL formats
    /// - `postgres://user:pass@localhost:5432/draox`
    /// - `postgresql://user:pass@host/db?sslmode=require`
    pub async fn new(config: &SqlConfig) -> Result<Self> {
        let pool = PgPoolOptions::new()
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

        info!("PostgreSQL storage initialized: {}", config.url);
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

impl StorageBackend for PostgresStorage {
    fn get(&self, namespace: &str, key: &str) -> BoxFuture<'_, Result<Option<serde_json::Value>>> {
        let namespace = namespace.to_owned();
        let key = key.to_owned();

        Box::pin(async move {
            let row = sqlx::query("SELECT value FROM kv_store WHERE namespace = $1 AND key = $2")
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
                r#"INSERT INTO kv_store (namespace, key, value, updated_at)
                   VALUES ($1, $2, $3, $4)
                   ON CONFLICT (namespace, key)
                   DO UPDATE SET value = EXCLUDED.value, updated_at = EXCLUDED.updated_at"#,
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
                sqlx::query("DELETE FROM kv_store WHERE namespace = $1 AND key = $2")
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
                "SELECT key FROM kv_store WHERE namespace = $1 AND key LIKE $2",
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

    /// Helper: skip test if POSTGRES_TEST_URL is not set.
    fn test_url() -> Option<String> {
        std::env::var("POSTGRES_TEST_URL").ok()
    }

    async fn make_storage() -> Option<PostgresStorage> {
        let url = test_url()?;
        let config = SqlConfig {
            url,
            max_connections: 2,
            min_connections: 1,
            idle_timeout_secs: 60,
            max_lifetime_secs: 300,
            run_migrations: true,
        };
        PostgresStorage::new(&config).await.ok()
    }

    /// Clean up test data in a specific namespace.
    async fn cleanup(storage: &PostgresStorage, namespace: &str) {
        let _ = sqlx::query("DELETE FROM kv_store WHERE namespace = $1")
            .bind(namespace)
            .execute(&storage.pool)
            .await;
    }

    #[tokio::test]
    #[ignore] // Requires: POSTGRES_TEST_URL=postgres://user:pass@localhost:5432/test_db
    async fn test_postgres_set_and_get() {
        let store = make_storage().await.expect("PostgreSQL not available");
        cleanup(&store, "pg_test").await;

        let val = json!({"name": "draox", "version": 1});
        store.set("pg_test", "key1", val.clone()).await.unwrap();

        let result = store.get("pg_test", "key1").await.unwrap();
        assert_eq!(result, Some(val));

        cleanup(&store, "pg_test").await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_postgres_get_missing() {
        let store = make_storage().await.expect("PostgreSQL not available");

        let result = store.get("pg_test", "nonexistent").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    #[ignore]
    async fn test_postgres_set_overwrite() {
        let store = make_storage().await.expect("PostgreSQL not available");
        cleanup(&store, "pg_test").await;

        store.set("pg_test", "key", json!(1)).await.unwrap();
        store.set("pg_test", "key", json!(2)).await.unwrap();

        let result = store.get("pg_test", "key").await.unwrap();
        assert_eq!(result, Some(json!(2)));

        cleanup(&store, "pg_test").await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_postgres_delete() {
        let store = make_storage().await.expect("PostgreSQL not available");
        cleanup(&store, "pg_test").await;

        store.set("pg_test", "key", json!("hello")).await.unwrap();
        let deleted = store.delete("pg_test", "key").await.unwrap();
        assert!(deleted);

        let result = store.get("pg_test", "key").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    #[ignore]
    async fn test_postgres_delete_missing() {
        let store = make_storage().await.expect("PostgreSQL not available");

        let deleted = store.delete("pg_test", "nope").await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    #[ignore]
    async fn test_postgres_list_keys() {
        let store = make_storage().await.expect("PostgreSQL not available");
        cleanup(&store, "pg_test").await;

        store.set("pg_test", "user:1", json!(1)).await.unwrap();
        store.set("pg_test", "user:2", json!(2)).await.unwrap();
        store.set("pg_test", "session:1", json!(3)).await.unwrap();

        let mut keys = store.list_keys("pg_test", "user:").await.unwrap();
        keys.sort();
        assert_eq!(keys, vec!["user:1", "user:2"]);

        cleanup(&store, "pg_test").await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_postgres_namespace_isolation() {
        let store = make_storage().await.expect("PostgreSQL not available");
        cleanup(&store, "pg_ns_a").await;
        cleanup(&store, "pg_ns_b").await;

        store.set("pg_ns_a", "shared_key", json!("alpha")).await.unwrap();
        store.set("pg_ns_b", "shared_key", json!("beta")).await.unwrap();

        let a = store.get("pg_ns_a", "shared_key").await.unwrap();
        let b = store.get("pg_ns_b", "shared_key").await.unwrap();

        assert_eq!(a, Some(json!("alpha")));
        assert_eq!(b, Some(json!("beta")));

        cleanup(&store, "pg_ns_a").await;
        cleanup(&store, "pg_ns_b").await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_postgres_json_values() {
        let store = make_storage().await.expect("PostgreSQL not available");
        cleanup(&store, "pg_test").await;

        let complex = json!({
            "users": [
                {"id": 1, "name": "Alice", "active": true},
                {"id": 2, "name": "Bob", "active": false}
            ],
            "metadata": {"version": "1.0", "count": 2}
        });

        store.set("pg_test", "complex", complex.clone()).await.unwrap();
        let result = store.get("pg_test", "complex").await.unwrap();
        assert_eq!(result, Some(complex));

        cleanup(&store, "pg_test").await;
    }
}
