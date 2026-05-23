use dashmap::DashMap;
use std::sync::Arc;
use tracing::{info, warn};
use crate::provider::{SecretValue, SecretsError, SecretsProvider, SecretsResult};

/// Central secrets manager — in-memory cache + backend provider.
/// Supports hot-reload without server restart.
pub struct SecretsManager {
    provider: Arc<dyn SecretsProvider>,
    /// In-memory cache to avoid repeated calls to the backend
    cache: Arc<DashMap<String, SecretValue>>,
}

impl SecretsManager {
    pub fn new(provider: Arc<dyn SecretsProvider>) -> Self {
        Self {
            provider,
            cache: Arc::new(DashMap::new()),
        }
    }

    /// Get a secret, using cache if available and not expired.
    pub async fn get(&self, name: &str) -> SecretsResult<SecretValue> {
        if let Some(cached) = self.cache.get(name) {
            if !cached.is_expired() {
                return Ok(cached.clone());
            }
            warn!(secret = %name, "cached secret expired, refreshing");
        }

        let secret = self.provider.get_secret(name).await?;
        self.cache.insert(name.to_string(), secret.clone());
        Ok(secret)
    }

    /// Get the string value of a secret directly.
    pub async fn get_value(&self, name: &str) -> SecretsResult<String> {
        self.get(name).await.map(|s| s.value)
    }

    /// Store a new secret.
    pub async fn set(&self, name: &str, value: &str) -> SecretsResult<()> {
        self.provider.put_secret(name, value).await?;
        self.cache.remove(name);
        info!(secret = %name, "secret stored");
        Ok(())
    }

    /// Delete a secret.
    pub async fn delete(&self, name: &str) -> SecretsResult<()> {
        self.provider.delete_secret(name).await?;
        self.cache.remove(name);
        Ok(())
    }

    /// Rotate a secret (provider generates new value).
    pub async fn rotate(&self, name: &str) -> SecretsResult<SecretValue> {
        let new_secret = self.provider.rotate_secret(name).await?;
        self.cache.insert(name.to_string(), new_secret.clone());
        info!(secret = %name, "secret rotated");
        Ok(new_secret)
    }

    /// Force-refresh the cache for a secret.
    pub async fn invalidate_cache(&self, name: &str) {
        self.cache.remove(name);
    }

    /// List all available secrets.
    pub async fn list(&self) -> SecretsResult<Vec<String>> {
        self.provider.list_secrets().await
    }

    /// Pre-load a set of secrets into the cache.
    pub async fn preload(&self, names: &[&str]) -> SecretsResult<()> {
        for name in names {
            if let Err(e) = self.get(name).await {
                warn!(secret = %name, error = %e, "failed to preload secret");
            }
        }
        Ok(())
    }
}
