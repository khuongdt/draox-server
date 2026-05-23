use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use server_core::ClientId;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DevicePlatform {
    Fcm,
    Apns,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceToken {
    pub client_id: ClientId,
    pub token: String,
    pub platform: DevicePlatform,
    pub registered_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
    pub app_version: Option<String>,
}

impl DeviceToken {
    pub fn new(client_id: ClientId, token: String, platform: DevicePlatform) -> Self {
        let now = Utc::now();
        Self {
            client_id,
            token,
            platform,
            registered_at: now,
            last_used_at: now,
            app_version: None,
        }
    }
}

/// In-memory device token registry. Back with Redis/DB in production.
pub struct DeviceTokenRegistry {
    // client_id -> Vec<DeviceToken>
    tokens: Arc<DashMap<String, Vec<DeviceToken>>>,
}

impl DeviceTokenRegistry {
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(DashMap::new()),
        }
    }

    pub fn register(&self, token: DeviceToken) {
        let mut entry = self.tokens.entry(token.client_id.as_str().to_string()).or_insert_with(Vec::new);
        // Replace existing token for same platform if present
        if let Some(pos) = entry.iter().position(|t| t.platform == token.platform && t.token == token.token) {
            entry[pos] = token;
        } else {
            entry.push(token);
        }
    }

    pub fn unregister(&self, client_id: &ClientId, token_str: &str) {
        if let Some(mut entry) = self.tokens.get_mut(client_id.as_str()) {
            entry.retain(|t| t.token != token_str);
        }
    }

    pub fn get_tokens(&self, client_id: &ClientId) -> Vec<DeviceToken> {
        self.tokens
            .get(client_id.as_str())
            .map(|entry| entry.clone())
            .unwrap_or_default()
    }

    pub fn get_tokens_for_platform(&self, client_id: &ClientId, platform: &DevicePlatform) -> Vec<DeviceToken> {
        self.get_tokens(client_id)
            .into_iter()
            .filter(|t| &t.platform == platform)
            .collect()
    }

    pub fn remove_all(&self, client_id: &ClientId) {
        self.tokens.remove(client_id.as_str());
    }

    pub fn mark_used(&self, client_id: &ClientId, token_str: &str) {
        if let Some(mut entry) = self.tokens.get_mut(client_id.as_str()) {
            for token in entry.iter_mut() {
                if token.token == token_str {
                    token.last_used_at = Utc::now();
                    break;
                }
            }
        }
    }
}

impl Default for DeviceTokenRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_retrieve() {
        let reg = DeviceTokenRegistry::new();
        let client = ClientId::from_str("cli_1");
        let token = DeviceToken::new(client.clone(), "fcm-token-abc".to_string(), DevicePlatform::Fcm);
        reg.register(token);
        let tokens = reg.get_tokens(&client);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token, "fcm-token-abc");
    }

    #[test]
    fn test_unregister() {
        let reg = DeviceTokenRegistry::new();
        let client = ClientId::from_str("cli_2");
        reg.register(DeviceToken::new(client.clone(), "tok".to_string(), DevicePlatform::Apns));
        reg.unregister(&client, "tok");
        assert!(reg.get_tokens(&client).is_empty());
    }
}
