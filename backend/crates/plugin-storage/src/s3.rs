use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client;
use aws_sdk_s3::config::{Credentials, Region};
use aws_sdk_s3::presigning::PresigningConfig;
use chrono::Utc;
use std::time::Duration;
use crate::provider::*;

pub struct S3Config {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub region: String,
    /// Override endpoint for R2/MinIO (e.g., "https://minio.local:9000")
    pub endpoint_url: Option<String>,
}

pub struct S3Backend {
    client: Client,
}

impl S3Backend {
    pub async fn new(config: S3Config) -> Self {
        let creds = Credentials::new(
            &config.access_key_id,
            &config.secret_access_key,
            None,
            None,
            "draox-storage",
        );
        let mut builder = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(config.region.clone()))
            .credentials_provider(creds);

        if let Some(endpoint) = config.endpoint_url {
            builder = builder.endpoint_url(endpoint);
        }

        let sdk_config = builder.load().await;
        let client = Client::new(&sdk_config);
        Self { client }
    }
}

#[async_trait]
impl ObjectStorageProvider for S3Backend {
    async fn put(&self, bucket: &str, params: &UploadParams, data: Vec<u8>) -> StorageResult<ObjectMetadata> {
        let size = data.len() as u64;
        self.client
            .put_object()
            .bucket(bucket)
            .key(&params.key)
            .content_type(&params.content_type)
            .body(data.into())
            .send()
            .await
            .map_err(|e| StorageError::UploadFailed(e.to_string()))?;

        Ok(ObjectMetadata {
            key: params.key.clone(),
            bucket: bucket.to_string(),
            size_bytes: size,
            content_type: params.content_type.clone(),
            etag: None,
            uploaded_at: Utc::now(),
            owner: params.owner.clone(),
        })
    }

    async fn get(&self, bucket: &str, key: &str) -> StorageResult<Vec<u8>> {
        let resp = self.client
            .get_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| StorageError::NotFound(e.to_string()))?;

        let bytes = resp
            .body
            .collect()
            .await
            .map_err(|e| StorageError::Provider(e.to_string()))?;

        Ok(bytes.into_bytes().to_vec())
    }

    async fn delete(&self, bucket: &str, key: &str) -> StorageResult<()> {
        self.client
            .delete_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| StorageError::Provider(e.to_string()))?;
        Ok(())
    }

    async fn list(&self, bucket: &str, prefix: &str) -> StorageResult<Vec<ObjectMetadata>> {
        let resp = self.client
            .list_objects_v2()
            .bucket(bucket)
            .prefix(prefix)
            .send()
            .await
            .map_err(|e| StorageError::Provider(e.to_string()))?;

        let objects = resp
            .contents
            .unwrap_or_default()
            .into_iter()
            .map(|obj| ObjectMetadata {
                key: obj.key.unwrap_or_default(),
                bucket: bucket.to_string(),
                size_bytes: obj.size.unwrap_or(0) as u64,
                content_type: "application/octet-stream".to_string(),
                etag: obj.e_tag,
                uploaded_at: Utc::now(),
                owner: None,
            })
            .collect();
        Ok(objects)
    }

    async fn head(&self, bucket: &str, key: &str) -> StorageResult<ObjectMetadata> {
        let resp = self.client
            .head_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| StorageError::NotFound(e.to_string()))?;

        Ok(ObjectMetadata {
            key: key.to_string(),
            bucket: bucket.to_string(),
            size_bytes: resp.content_length.unwrap_or(0) as u64,
            content_type: resp.content_type.unwrap_or_default(),
            etag: resp.e_tag,
            uploaded_at: Utc::now(),
            owner: None,
        })
    }

    async fn presigned_put_url(
        &self,
        bucket: &str,
        key: &str,
        expires_secs: u64,
        content_type: &str,
    ) -> StorageResult<String> {
        let config = PresigningConfig::expires_in(Duration::from_secs(expires_secs))
            .map_err(|e| StorageError::Provider(e.to_string()))?;

        let presigned = self.client
            .put_object()
            .bucket(bucket)
            .key(key)
            .content_type(content_type)
            .presigned(config)
            .await
            .map_err(|e| StorageError::Provider(e.to_string()))?;

        Ok(presigned.uri().to_string())
    }

    async fn presigned_get_url(
        &self,
        bucket: &str,
        key: &str,
        expires_secs: u64,
    ) -> StorageResult<String> {
        let config = PresigningConfig::expires_in(Duration::from_secs(expires_secs))
            .map_err(|e| StorageError::Provider(e.to_string()))?;

        let presigned = self.client
            .get_object()
            .bucket(bucket)
            .key(key)
            .presigned(config)
            .await
            .map_err(|e| StorageError::Provider(e.to_string()))?;

        Ok(presigned.uri().to_string())
    }
}
