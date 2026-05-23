use crate::auth::AdminRole;
use data_store::StorageBackend;
use serde::{Deserialize, Serialize};
use server_core::Result;
use std::sync::Arc;

const NAMESPACE: &str = "admin_users";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminUser {
    pub username: String,
    pub password_hash: String,
    pub role: AdminRole,
    #[serde(default)]
    pub banned: bool,
}

pub struct AdminUserStore {
    storage: Arc<dyn StorageBackend>,
}

impl AdminUserStore {
    pub fn new(storage: Arc<dyn StorageBackend>) -> Self {
        Self { storage }
    }

    pub async fn get(&self, username: &str) -> Option<AdminUser> {
        let value = self.storage.get(NAMESPACE, username).await.ok()??;
        serde_json::from_value(value).ok()
    }

    pub async fn set(&self, user: &AdminUser) -> Result<()> {
        let value = serde_json::to_value(user)
            .map_err(|e| server_core::Error::Storage(e.to_string()))?;
        self.storage.set(NAMESPACE, &user.username, value).await
    }

    pub async fn exists(&self, username: &str) -> bool {
        self.storage
            .get(NAMESPACE, username)
            .await
            .ok()
            .flatten()
            .is_some()
    }

    pub async fn list(&self) -> Vec<AdminUser> {
        let keys = match self.storage.list_keys(NAMESPACE, "").await {
            Ok(k) => k,
            Err(_) => return vec![],
        };
        let mut users = Vec::with_capacity(keys.len());
        for key in keys {
            if let Some(user) = self.get(&key).await {
                users.push(user);
            }
        }
        users
    }

    pub async fn delete(&self, username: &str) -> server_core::Result<bool> {
        self.storage.delete(NAMESPACE, username).await
    }
}
