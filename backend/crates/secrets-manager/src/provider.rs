use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum SecretsError {
    #[error("secret not found: {0}")]
    NotFound(String),
    #[error("access denied")]
    AccessDenied,
    #[error("provider error: {0}")]
    Provider(String),
    #[error("encryption error: {0}")]
    Encryption(String),
}

pub type SecretsResult<T> = Result<T, SecretsError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretValue {
    pub name: String,
    pub value: String,
    pub version: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl SecretValue {
    pub fn is_expired(&self) -> bool {
        self.expires_at.map(|e| e < Utc::now()).unwrap_or(false)
    }
}

/// Abstraction over different secrets backends.
#[async_trait]
pub trait SecretsProvider: Send + Sync + 'static {
    /// Retrieve a secret by name.
    async fn get_secret(&self, name: &str) -> SecretsResult<SecretValue>;

    /// Store or update a secret.
    async fn put_secret(&self, name: &str, value: &str) -> SecretsResult<()>;

    /// Delete a secret.
    async fn delete_secret(&self, name: &str) -> SecretsResult<()>;

    /// List all secret names.
    async fn list_secrets(&self) -> SecretsResult<Vec<String>>;

    /// Rotate a secret (provider generates a new value and stores it).
    async fn rotate_secret(&self, name: &str) -> SecretsResult<SecretValue>;
}
