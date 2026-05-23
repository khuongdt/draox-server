use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde_json::json;
use crate::provider::*;

/// HashiCorp Vault KV v2 backend.
pub struct VaultProvider {
    client: Client,
    address: String,
    token: String,
    mount: String,
}

impl VaultProvider {
    pub fn new(address: String, token: String, mount: String) -> Self {
        Self {
            client: Client::new(),
            address,
            token,
            mount,
        }
    }

    fn secret_url(&self, name: &str) -> String {
        format!("{}/v1/{}/data/{}", self.address, self.mount, name)
    }

    fn metadata_url(&self, name: &str) -> String {
        format!("{}/v1/{}/metadata/{}", self.address, self.mount, name)
    }

    fn list_url(&self) -> String {
        format!("{}/v1/{}/metadata/", self.address, self.mount)
    }
}

#[async_trait]
impl SecretsProvider for VaultProvider {
    async fn get_secret(&self, name: &str) -> SecretsResult<SecretValue> {
        let resp: serde_json::Value = self
            .client
            .get(self.secret_url(name))
            .header("X-Vault-Token", &self.token)
            .send()
            .await
            .map_err(|e| SecretsError::Provider(e.to_string()))?
            .json()
            .await
            .map_err(|e| SecretsError::Provider(e.to_string()))?;

        let value = resp["data"]["data"]["value"]
            .as_str()
            .ok_or_else(|| SecretsError::NotFound(name.to_string()))?
            .to_string();

        let version = resp["data"]["metadata"]["version"].as_u64().map(|v| v.to_string());

        Ok(SecretValue {
            name: name.to_string(),
            value,
            version,
            created_at: Some(Utc::now()),
            expires_at: None,
        })
    }

    async fn put_secret(&self, name: &str, value: &str) -> SecretsResult<()> {
        let body = json!({ "data": { "value": value } });
        let resp = self
            .client
            .post(self.secret_url(name))
            .header("X-Vault-Token", &self.token)
            .json(&body)
            .send()
            .await
            .map_err(|e| SecretsError::Provider(e.to_string()))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(SecretsError::Provider(format!("vault put failed: {}", resp.status())))
        }
    }

    async fn delete_secret(&self, name: &str) -> SecretsResult<()> {
        self.client
            .delete(self.metadata_url(name))
            .header("X-Vault-Token", &self.token)
            .send()
            .await
            .map_err(|e| SecretsError::Provider(e.to_string()))?;
        Ok(())
    }

    async fn list_secrets(&self) -> SecretsResult<Vec<String>> {
        let resp: serde_json::Value = self
            .client
            .request(reqwest::Method::from_bytes(b"LIST").unwrap(), self.list_url())
            .header("X-Vault-Token", &self.token)
            .send()
            .await
            .map_err(|e| SecretsError::Provider(e.to_string()))?
            .json()
            .await
            .map_err(|e| SecretsError::Provider(e.to_string()))?;

        let keys = resp["data"]["keys"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        Ok(keys)
    }

    async fn rotate_secret(&self, name: &str) -> SecretsResult<SecretValue> {
        // For Vault, rotation means generating a new random value and storing it
        let new_value = generate_random_secret(32);
        self.put_secret(name, &new_value).await?;
        self.get_secret(name).await
    }
}

fn generate_random_secret(len: usize) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
        .collect()
}
