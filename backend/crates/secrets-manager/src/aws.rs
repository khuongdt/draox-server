use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use crate::provider::*;

/// AWS Secrets Manager backend (via REST API).
/// In production, use the aws-sdk-secretsmanager crate for full support.
pub struct AwsSecretsProvider {
    client: Client,
    region: String,
    access_key_id: String,
    secret_access_key: String,
}

impl AwsSecretsProvider {
    pub fn new(region: String, access_key_id: String, secret_access_key: String) -> Self {
        Self {
            client: Client::new(),
            region,
            access_key_id,
            secret_access_key,
        }
    }
}

#[async_trait]
impl SecretsProvider for AwsSecretsProvider {
    async fn get_secret(&self, name: &str) -> SecretsResult<SecretValue> {
        // AWS Secrets Manager endpoint via HTTPS API
        let url = format!(
            "https://secretsmanager.{}.amazonaws.com",
            self.region
        );
        let body = serde_json::json!({ "SecretId": name });
        let resp: serde_json::Value = self
            .client
            .post(&url)
            .header("X-Amz-Target", "secretsmanager.GetSecretValue")
            .header("Content-Type", "application/x-amz-json-1.1")
            // Note: In production, use AWS SigV4 signing
            .json(&body)
            .send()
            .await
            .map_err(|e| SecretsError::Provider(e.to_string()))?
            .json()
            .await
            .map_err(|e| SecretsError::Provider(e.to_string()))?;

        let value = resp["SecretString"]
            .as_str()
            .ok_or_else(|| SecretsError::NotFound(name.to_string()))?
            .to_string();

        Ok(SecretValue {
            name: name.to_string(),
            value,
            version: resp["VersionId"].as_str().map(|s| s.to_string()),
            created_at: Some(Utc::now()),
            expires_at: None,
        })
    }

    async fn put_secret(&self, name: &str, value: &str) -> SecretsResult<()> {
        let url = format!("https://secretsmanager.{}.amazonaws.com", self.region);
        let body = serde_json::json!({
            "SecretId": name,
            "SecretString": value
        });
        let resp = self
            .client
            .post(&url)
            .header("X-Amz-Target", "secretsmanager.PutSecretValue")
            .header("Content-Type", "application/x-amz-json-1.1")
            .json(&body)
            .send()
            .await
            .map_err(|e| SecretsError::Provider(e.to_string()))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(SecretsError::Provider(format!("AWS SM put failed: {}", resp.status())))
        }
    }

    async fn delete_secret(&self, name: &str) -> SecretsResult<()> {
        let url = format!("https://secretsmanager.{}.amazonaws.com", self.region);
        let body = serde_json::json!({
            "SecretId": name,
            "ForceDeleteWithoutRecovery": false
        });
        self.client
            .post(&url)
            .header("X-Amz-Target", "secretsmanager.DeleteSecret")
            .header("Content-Type", "application/x-amz-json-1.1")
            .json(&body)
            .send()
            .await
            .map_err(|e| SecretsError::Provider(e.to_string()))?;
        Ok(())
    }

    async fn list_secrets(&self) -> SecretsResult<Vec<String>> {
        let url = format!("https://secretsmanager.{}.amazonaws.com", self.region);
        let resp: serde_json::Value = self
            .client
            .post(&url)
            .header("X-Amz-Target", "secretsmanager.ListSecrets")
            .header("Content-Type", "application/x-amz-json-1.1")
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| SecretsError::Provider(e.to_string()))?
            .json()
            .await
            .map_err(|e| SecretsError::Provider(e.to_string()))?;

        let names = resp["SecretList"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s["Name"].as_str().map(|n| n.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(names)
    }

    async fn rotate_secret(&self, name: &str) -> SecretsResult<SecretValue> {
        let url = format!("https://secretsmanager.{}.amazonaws.com", self.region);
        let body = serde_json::json!({ "SecretId": name });
        self.client
            .post(&url)
            .header("X-Amz-Target", "secretsmanager.RotateSecret")
            .header("Content-Type", "application/x-amz-json-1.1")
            .json(&body)
            .send()
            .await
            .map_err(|e| SecretsError::Provider(e.to_string()))?;

        self.get_secret(name).await
    }
}
