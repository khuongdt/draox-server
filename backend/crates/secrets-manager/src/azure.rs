use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use crate::provider::*;

/// Azure Key Vault backend.
pub struct AzureKeyVaultProvider {
    client: Client,
    vault_url: String,
    access_token: String,
}

impl AzureKeyVaultProvider {
    pub fn new(vault_url: String, access_token: String) -> Self {
        Self {
            client: Client::new(),
            vault_url: vault_url.trim_end_matches('/').to_string(),
            access_token,
        }
    }

    fn secret_url(&self, name: &str) -> String {
        format!("{}/secrets/{}?api-version=7.4", self.vault_url, name)
    }

    fn secrets_list_url(&self) -> String {
        format!("{}/secrets?api-version=7.4", self.vault_url)
    }
}

#[async_trait]
impl SecretsProvider for AzureKeyVaultProvider {
    async fn get_secret(&self, name: &str) -> SecretsResult<SecretValue> {
        let resp: serde_json::Value = self
            .client
            .get(self.secret_url(name))
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| SecretsError::Provider(e.to_string()))?
            .json()
            .await
            .map_err(|e| SecretsError::Provider(e.to_string()))?;

        let value = resp["value"]
            .as_str()
            .ok_or_else(|| SecretsError::NotFound(name.to_string()))?
            .to_string();

        Ok(SecretValue {
            name: name.to_string(),
            value,
            version: None,
            created_at: Some(Utc::now()),
            expires_at: resp["attributes"]["exp"]
                .as_i64()
                .map(|ts| chrono::DateTime::from_timestamp(ts, 0).unwrap_or(Utc::now())),
        })
    }

    async fn put_secret(&self, name: &str, value: &str) -> SecretsResult<()> {
        let body = serde_json::json!({ "value": value });
        let resp = self
            .client
            .put(self.secret_url(name))
            .bearer_auth(&self.access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| SecretsError::Provider(e.to_string()))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(SecretsError::Provider(format!("Azure KV put failed: {}", resp.status())))
        }
    }

    async fn delete_secret(&self, name: &str) -> SecretsResult<()> {
        self.client
            .delete(self.secret_url(name))
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| SecretsError::Provider(e.to_string()))?;
        Ok(())
    }

    async fn list_secrets(&self) -> SecretsResult<Vec<String>> {
        let resp: serde_json::Value = self
            .client
            .get(self.secrets_list_url())
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| SecretsError::Provider(e.to_string()))?
            .json()
            .await
            .map_err(|e| SecretsError::Provider(e.to_string()))?;

        let names = resp["value"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| {
                        s["id"].as_str().and_then(|id| id.split('/').last().map(|n| n.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(names)
    }

    async fn rotate_secret(&self, name: &str) -> SecretsResult<SecretValue> {
        let url = format!("{}/secrets/{}/rotate?api-version=7.4", self.vault_url, name);
        self.client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| SecretsError::Provider(e.to_string()))?;

        self.get_secret(name).await
    }
}
