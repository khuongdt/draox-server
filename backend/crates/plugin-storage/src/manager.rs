use std::sync::Arc;
use crate::presigned::{PresignedUploadRequest, PresignedUploadResponse, validate_content_type};
use crate::provider::{ObjectMetadata, ObjectStorageProvider, StorageError, StorageResult, UploadParams};
use crate::quota::QuotaManager;

const DEFAULT_PRESIGN_EXPIRES_SECS: u64 = 900; // 15 minutes

pub struct StorageManager {
    provider: Arc<dyn ObjectStorageProvider>,
    quota: Arc<QuotaManager>,
    default_bucket: String,
    allowed_content_types: Vec<String>,
}

impl StorageManager {
    pub fn new(
        provider: Arc<dyn ObjectStorageProvider>,
        default_bucket: String,
        allowed_content_types: Vec<String>,
    ) -> Self {
        Self {
            provider,
            quota: Arc::new(QuotaManager::new()),
            default_bucket,
            allowed_content_types,
        }
    }

    pub fn with_quota(mut self, quota: QuotaManager) -> Self {
        self.quota = Arc::new(quota);
        self
    }

    /// Generate a presigned URL for client-direct upload.
    pub async fn create_presigned_upload(
        &self,
        request: PresignedUploadRequest,
    ) -> StorageResult<PresignedUploadResponse> {
        let allowed: Vec<&str> = self.allowed_content_types.iter().map(|s| s.as_str()).collect();
        if !validate_content_type(&request.content_type, &allowed) {
            return Err(StorageError::InvalidContentType(request.content_type.clone()));
        }

        if let Some(owner) = &request.owner {
            if let Some(max_size) = request.max_size_bytes {
                self.quota.check(owner, max_size)?;
            }
        }

        let bucket = if request.bucket.is_empty() {
            &self.default_bucket
        } else {
            &request.bucket
        };

        let url = self
            .provider
            .presigned_put_url(bucket, &request.key, request.expires_secs, &request.content_type)
            .await?;

        Ok(PresignedUploadResponse {
            upload_url: url,
            key: request.key,
            bucket: bucket.to_string(),
            expires_in_secs: request.expires_secs,
            content_type: request.content_type,
        })
    }

    /// Generate a presigned download URL.
    pub async fn create_presigned_download(
        &self,
        bucket: Option<&str>,
        key: &str,
        expires_secs: Option<u64>,
    ) -> StorageResult<String> {
        let bucket = bucket.unwrap_or(&self.default_bucket);
        let expires = expires_secs.unwrap_or(DEFAULT_PRESIGN_EXPIRES_SECS);
        self.provider.presigned_get_url(bucket, key, expires).await
    }

    /// Server-side upload (for small files processed by the server).
    pub async fn upload(
        &self,
        bucket: Option<&str>,
        key: String,
        content_type: String,
        data: Vec<u8>,
        owner: Option<String>,
    ) -> StorageResult<ObjectMetadata> {
        let bucket = bucket.unwrap_or(&self.default_bucket);
        let size = data.len() as u64;

        if let Some(ref o) = owner {
            self.quota.check(o, size)?;
        }

        let params = UploadParams {
            key,
            content_type,
            size_bytes: Some(size),
            owner: owner.clone(),
            metadata: Default::default(),
        };

        let meta = self.provider.put(bucket, &params, data).await?;

        if let Some(o) = owner {
            self.quota.add_usage(&o, size);
        }

        Ok(meta)
    }

    pub async fn delete(&self, bucket: Option<&str>, key: &str, owner: Option<&str>) -> StorageResult<()> {
        let bucket = bucket.unwrap_or(&self.default_bucket);
        // Get size before delete for quota accounting
        let size = self.provider.head(bucket, key).await.map(|m| m.size_bytes).unwrap_or(0);
        self.provider.delete(bucket, key).await?;
        if let Some(o) = owner {
            self.quota.remove_usage(o, size);
        }
        Ok(())
    }

    pub fn quota(&self) -> &QuotaManager {
        &self.quota
    }
}
