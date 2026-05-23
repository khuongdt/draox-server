use async_trait::async_trait;
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::warn;
use crate::provider::{PushError, PushNotification, PushProvider, PushResult};

const APNS_PRODUCTION: &str = "https://api.push.apple.com";
const APNS_SANDBOX: &str = "https://api.sandbox.push.apple.com";

#[derive(Debug, Serialize, Deserialize)]
struct ApnsClaims {
    iss: String,
    iat: i64,
}

pub struct ApnsProvider {
    team_id: String,
    key_id: String,
    bundle_id: String,
    encoding_key: EncodingKey,
    endpoint: &'static str,
    client: Client,
}

impl ApnsProvider {
    pub fn new_production(
        team_id: String,
        key_id: String,
        bundle_id: String,
        private_key_pem: &str,
    ) -> Result<Self, String> {
        let encoding_key = EncodingKey::from_ec_pem(private_key_pem.as_bytes())
            .map_err(|e| e.to_string())?;
        Ok(Self {
            team_id,
            key_id,
            bundle_id,
            encoding_key,
            endpoint: APNS_PRODUCTION,
            client: Client::builder().http2_prior_knowledge().build().unwrap(),
        })
    }

    pub fn new_sandbox(
        team_id: String,
        key_id: String,
        bundle_id: String,
        private_key_pem: &str,
    ) -> Result<Self, String> {
        let encoding_key = EncodingKey::from_ec_pem(private_key_pem.as_bytes())
            .map_err(|e| e.to_string())?;
        Ok(Self {
            team_id,
            key_id,
            bundle_id,
            encoding_key,
            endpoint: APNS_SANDBOX,
            client: Client::builder().http2_prior_knowledge().build().unwrap(),
        })
    }

    fn jwt_token(&self) -> PushResult<String> {
        let claims = ApnsClaims {
            iss: self.team_id.clone(),
            iat: Utc::now().timestamp(),
        };
        let mut header = Header::new(jsonwebtoken::Algorithm::ES256);
        header.kid = Some(self.key_id.clone());
        encode(&header, &claims, &self.encoding_key)
            .map_err(|e| PushError::Provider(e.to_string()))
    }
}

#[async_trait]
impl PushProvider for ApnsProvider {
    async fn send(&self, device_token: &str, notification: &PushNotification) -> PushResult<()> {
        let jwt = self.jwt_token()?;
        let url = format!("{}/3/device/{}", self.endpoint, device_token);

        let mut aps = json!({
            "alert": {
                "title": notification.title,
                "body": notification.body,
            }
        });
        if let Some(badge) = notification.badge {
            aps["badge"] = json!(badge);
        }
        if let Some(ref sound) = notification.sound {
            aps["sound"] = json!(sound);
        }

        let payload = json!({ "aps": aps });

        let resp = self
            .client
            .post(&url)
            .header("authorization", format!("bearer {}", jwt))
            .header("apns-topic", &self.bundle_id)
            .header("apns-push-type", "alert")
            .json(&payload)
            .send()
            .await
            .map_err(|e| PushError::Provider(e.to_string()))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            if status.as_u16() == 429 {
                Err(PushError::RateLimited)
            } else if text.contains("BadDeviceToken") {
                Err(PushError::InvalidToken)
            } else {
                Err(PushError::Provider(format!("APNs error {status}: {text}")))
            }
        }
    }

    async fn send_batch(
        &self,
        tokens: &[String],
        notification: &PushNotification,
    ) -> Vec<(String, PushResult<()>)> {
        let mut results = Vec::with_capacity(tokens.len());
        for token in tokens {
            let result = self.send(token, notification).await;
            if let Err(ref e) = result {
                warn!(token = %token, error = %e, "APNs send failed");
            }
            results.push((token.clone(), result));
        }
        results
    }

    fn platform(&self) -> &str {
        "apns"
    }
}
