// Phase 5: Data Store — SQL + NoSQL storage

pub mod backend;
mod error;
pub mod mongodb;
pub mod mysql;
pub mod postgres;
pub mod routing;
pub mod schema;
pub mod sqlite;
pub mod transaction;

pub use backend::{BoxFuture, StorageBackend};
pub use self::mongodb::MongoStorage;
pub use mysql::MySqlStorage;
pub use postgres::PostgresStorage;
pub use routing::ReadReplicaRouter;
pub use schema::{SchemaDefinition, SCHEMAS, find_schema};
pub use sqlite::SqliteStorage;
pub use transaction::{Transaction, TransactionOp, execute_transaction};

use server_config::model::StorageConfig;
use std::sync::Arc;
use tracing::info;

/// Factory: create a storage backend from configuration.
///
/// Selects the backend based on `config.backend`:
/// - `"sqlite"` (default) → [`SqliteStorage`]
/// - `"postgres"` / `"postgresql"` → [`PostgresStorage`]
/// - `"mysql"` / `"mariadb"` → [`MySqlStorage`]
/// - `"mongodb"` → [`MongoStorage`]
pub async fn create_storage_backend(
    config: &StorageConfig,
) -> server_core::Result<Arc<dyn StorageBackend>> {
    match config.backend.as_str() {
        "sqlite" => {
            let storage = SqliteStorage::from_config(&config.sql).await?;
            info!("storage backend: SQLite ({})", config.sql.url);
            Ok(Arc::new(storage))
        }
        "postgres" | "postgresql" => {
            let storage = PostgresStorage::new(&config.sql).await?;
            info!("storage backend: PostgreSQL ({})", config.sql.url);
            Ok(Arc::new(storage))
        }
        "mysql" | "mariadb" => {
            let storage = MySqlStorage::new(&config.sql).await?;
            info!("storage backend: MySQL ({})", config.sql.url);
            Ok(Arc::new(storage))
        }
        "mongodb" => {
            let storage = MongoStorage::new(&config.mongodb).await?;
            info!("storage backend: MongoDB ({})", config.mongodb.url);
            Ok(Arc::new(storage))
        }
        other => Err(server_core::Error::Config(format!(
            "unknown storage backend: '{other}'. Supported: sqlite, postgres, mysql, mariadb, mongodb"
        ))),
    }
}
