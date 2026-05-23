use mongodb::bson::{doc, Bson, Document};
use mongodb::options::{ClientOptions, IndexOptions};
use mongodb::{Client, Collection, IndexModel};
use server_config::model::MongoConfig;
use server_core::Result;
use tracing::info;

use crate::backend::{BoxFuture, StorageBackend};
use crate::error::into_mongo_error;

/// MongoDB-backed key-value storage.
///
/// Stores JSON values as native BSON documents in a `kv_store` collection,
/// keyed by a compound unique index on `(namespace, key)`.
/// Suitable for deployments requiring flexible document storage with rich querying.
pub struct MongoStorage {
    collection: Collection<Document>,
}

impl MongoStorage {
    /// Connect to a MongoDB database using the provided config.
    ///
    /// Creates the `kv_store` collection (implicitly) and ensures a unique compound
    /// index on `(namespace, key)` for upsert correctness.
    ///
    /// # Example config
    /// ```toml
    /// [storage.mongodb]
    /// enabled = true
    /// url = "mongodb://localhost:27017"
    /// database = "draox"
    /// max_pool_size = 10
    /// ```
    pub async fn new(config: &MongoConfig) -> Result<Self> {
        let mut options = ClientOptions::parse(&config.url)
            .await
            .map_err(into_mongo_error)?;
        options.max_pool_size = Some(config.max_pool_size);

        let client = Client::with_options(options).map_err(into_mongo_error)?;
        let db = client.database(&config.database);
        let collection = db.collection::<Document>("kv_store");

        // Ensure unique compound index on (namespace, key).
        let index = IndexModel::builder()
            .keys(doc! { "namespace": 1, "key": 1 })
            .options(
                IndexOptions::builder()
                    .unique(true)
                    .name("namespace_key_unique".to_string())
                    .build(),
            )
            .build();
        collection
            .create_index(index)
            .await
            .map_err(into_mongo_error)?;

        info!(
            "MongoDB storage initialized: {} (db: {})",
            config.url, config.database
        );
        Ok(Self { collection })
    }
}

impl StorageBackend for MongoStorage {
    fn get(&self, namespace: &str, key: &str) -> BoxFuture<'_, Result<Option<serde_json::Value>>> {
        let namespace = namespace.to_owned();
        let key = key.to_owned();

        Box::pin(async move {
            let filter = doc! { "namespace": &namespace, "key": &key };
            let result = self
                .collection
                .find_one(filter)
                .await
                .map_err(into_mongo_error)?;

            match result {
                Some(document) => {
                    let bson_value = document
                        .get("value")
                        .cloned()
                        .unwrap_or(Bson::Null);
                    let json_value: serde_json::Value =
                        mongodb::bson::from_bson(bson_value).map_err(|e| {
                            server_core::Error::Storage(format!("BSON→JSON conversion: {e}"))
                        })?;
                    Ok(Some(json_value))
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
            let bson_value =
                mongodb::bson::to_bson(&value).map_err(|e| {
                    server_core::Error::Storage(format!("JSON→BSON conversion: {e}"))
                })?;
            let now = chrono::Utc::now().to_rfc3339();

            let filter = doc! { "namespace": &namespace, "key": &key };
            let update = doc! {
                "$set": {
                    "namespace": &namespace,
                    "key": &key,
                    "value": bson_value,
                    "updated_at": &now,
                }
            };

            self.collection
                .update_one(filter, update)
                .upsert(true)
                .await
                .map_err(into_mongo_error)?;

            Ok(())
        })
    }

    fn delete(&self, namespace: &str, key: &str) -> BoxFuture<'_, Result<bool>> {
        let namespace = namespace.to_owned();
        let key = key.to_owned();

        Box::pin(async move {
            let filter = doc! { "namespace": &namespace, "key": &key };
            let result = self
                .collection
                .delete_one(filter)
                .await
                .map_err(into_mongo_error)?;

            Ok(result.deleted_count > 0)
        })
    }

    fn list_keys(&self, namespace: &str, prefix: &str) -> BoxFuture<'_, Result<Vec<String>>> {
        let namespace = namespace.to_owned();
        let prefix = prefix.to_owned();

        Box::pin(async move {
            // Escape regex special characters in prefix, then anchor to start.
            let escaped = regex_escape(&prefix);
            let pattern = format!("^{escaped}");

            let filter = doc! {
                "namespace": &namespace,
                "key": { "$regex": &pattern }
            };

            let mut cursor = self
                .collection
                .find(filter)
                .projection(doc! { "key": 1, "_id": 0 })
                .await
                .map_err(into_mongo_error)?;

            let mut keys = Vec::new();
            use futures_util::TryStreamExt;
            while let Some(document) = cursor.try_next().await.map_err(into_mongo_error)? {
                if let Some(Bson::String(k)) = document.get("key") {
                    keys.push(k.clone());
                }
            }
            Ok(keys)
        })
    }
}

/// Escape regex metacharacters so a user-provided prefix is treated literally.
fn regex_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        if ".*+?^${}()|[]\\".contains(ch) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Helper: skip test if MONGO_TEST_URL is not set.
    fn test_url() -> Option<String> {
        std::env::var("MONGO_TEST_URL").ok()
    }

    async fn make_storage() -> Option<MongoStorage> {
        let url = test_url()?;
        let config = MongoConfig {
            enabled: true,
            url,
            database: "draox_test".to_string(),
            max_pool_size: 2,
        };
        MongoStorage::new(&config).await.ok()
    }

    /// Clean up test data in a specific namespace.
    async fn cleanup(storage: &MongoStorage, namespace: &str) {
        let _ = storage
            .collection
            .delete_many(doc! { "namespace": namespace })
            .await;
    }

    #[tokio::test]
    #[ignore] // Requires: MONGO_TEST_URL=mongodb://localhost:27017
    async fn test_mongo_set_and_get() {
        let store = make_storage().await.expect("MongoDB not available");
        cleanup(&store, "mongo_test").await;

        let val = json!({"name": "draox", "version": 1});
        store.set("mongo_test", "key1", val.clone()).await.unwrap();

        let result = store.get("mongo_test", "key1").await.unwrap();
        assert_eq!(result, Some(val));

        cleanup(&store, "mongo_test").await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_mongo_get_missing() {
        let store = make_storage().await.expect("MongoDB not available");

        let result = store.get("mongo_test", "nonexistent").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    #[ignore]
    async fn test_mongo_set_overwrite() {
        let store = make_storage().await.expect("MongoDB not available");
        cleanup(&store, "mongo_test").await;

        store.set("mongo_test", "key", json!(1)).await.unwrap();
        store.set("mongo_test", "key", json!(2)).await.unwrap();

        let result = store.get("mongo_test", "key").await.unwrap();
        assert_eq!(result, Some(json!(2)));

        cleanup(&store, "mongo_test").await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_mongo_delete() {
        let store = make_storage().await.expect("MongoDB not available");
        cleanup(&store, "mongo_test").await;

        store
            .set("mongo_test", "key", json!("hello"))
            .await
            .unwrap();
        let deleted = store.delete("mongo_test", "key").await.unwrap();
        assert!(deleted);

        let result = store.get("mongo_test", "key").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    #[ignore]
    async fn test_mongo_delete_missing() {
        let store = make_storage().await.expect("MongoDB not available");

        let deleted = store.delete("mongo_test", "nope").await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    #[ignore]
    async fn test_mongo_list_keys() {
        let store = make_storage().await.expect("MongoDB not available");
        cleanup(&store, "mongo_test").await;

        store.set("mongo_test", "user:1", json!(1)).await.unwrap();
        store.set("mongo_test", "user:2", json!(2)).await.unwrap();
        store
            .set("mongo_test", "session:1", json!(3))
            .await
            .unwrap();

        let mut keys = store.list_keys("mongo_test", "user:").await.unwrap();
        keys.sort();
        assert_eq!(keys, vec!["user:1", "user:2"]);

        cleanup(&store, "mongo_test").await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_mongo_namespace_isolation() {
        let store = make_storage().await.expect("MongoDB not available");
        cleanup(&store, "mongo_ns_a").await;
        cleanup(&store, "mongo_ns_b").await;

        store
            .set("mongo_ns_a", "shared_key", json!("alpha"))
            .await
            .unwrap();
        store
            .set("mongo_ns_b", "shared_key", json!("beta"))
            .await
            .unwrap();

        let a = store.get("mongo_ns_a", "shared_key").await.unwrap();
        let b = store.get("mongo_ns_b", "shared_key").await.unwrap();

        assert_eq!(a, Some(json!("alpha")));
        assert_eq!(b, Some(json!("beta")));

        cleanup(&store, "mongo_ns_a").await;
        cleanup(&store, "mongo_ns_b").await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_mongo_json_values() {
        let store = make_storage().await.expect("MongoDB not available");
        cleanup(&store, "mongo_test").await;

        let complex = json!({
            "users": [
                {"id": 1, "name": "Alice", "active": true},
                {"id": 2, "name": "Bob", "active": false}
            ],
            "metadata": {"version": "1.0", "count": 2}
        });

        store
            .set("mongo_test", "complex", complex.clone())
            .await
            .unwrap();
        let result = store.get("mongo_test", "complex").await.unwrap();
        assert_eq!(result, Some(complex));

        cleanup(&store, "mongo_test").await;
    }
}
