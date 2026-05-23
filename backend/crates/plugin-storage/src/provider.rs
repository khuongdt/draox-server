use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("object not found: {0}")]
    NotFound(String),
    #[error("quota exceeded")]
    QuotaExceeded,
    #[error("invalid content type: {0}")]
    InvalidContentType(String),
    #[error("upload failed: {0}")]
    UploadFailed(String),
    #[error("provider error: {0}")]
    Provider(String),
}

pub type StorageResult<T> = Result<T, StorageError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectMetadata {
    pub key: String,
    pub bucket: String,
    pub size_bytes: u64,
    pub content_type: String,
    pub etag: Option<String>,
    pub uploaded_at: DateTime<Utc>,
    pub owner: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadParams {
    pub key: String,
    pub content_type: String,
    pub size_bytes: Option<u64>,
    pub owner: Option<String>,
    pub metadata: std::collections::HashMap<String, String>,
}

/// Abstraction over S3/R2/MinIO storage backends.
#[async_trait]
pub trait ObjectStorageProvider: Send + Sync + 'static {
    /// Upload an object with raw bytes.
    async fn put(&self, bucket: &str, params: &UploadParams, data: Vec<u8>) -> StorageResult<ObjectMetadata>;

    /// Download an object.
    async fn get(&self, bucket: &str, key: &str) -> StorageResult<Vec<u8>>;

    /// Delete an object.
    async fn delete(&self, bucket: &str, key: &str) -> StorageResult<()>;

    /// List objects with a prefix.
    async fn list(&self, bucket: &str, prefix: &str) -> StorageResult<Vec<ObjectMetadata>>;

    /// Get object metadata without downloading.
    async fn head(&self, bucket: &str, key: &str) -> StorageResult<ObjectMetadata>;

    /// Generate a pre-signed URL for direct client upload (PUT).
    async fn presigned_put_url(
        &self,
        bucket: &str,
        key: &str,
        expires_secs: u64,
        content_type: &str,
    ) -> StorageResult<String>;

    /// Generate a pre-signed URL for direct client download (GET).
    async fn presigned_get_url(
        &self,
        bucket: &str,
        key: &str,
        expires_secs: u64,
    ) -> StorageResult<String>;
}
