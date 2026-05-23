use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use tracing::warn;
use crate::provider::{PushError, PushNotification, PushProvider, PushResult};

const FCM_URL: &str = "https://fcm.googleapis.com/v1/projects/{project_id}/messages:send";

pub struct FcmProvider {
    project_id: String,
    server_key: String,
    client: Client,
}

impl FcmProvider {
    pub fn new(project_id: String, server_key: String) -> Self {
        Self {
            project_id,
            server_key,
            client: Client::new(),
        }
    }

    fn endpoint(&self) -> String {
        FCM_URL.replace("{project_id}", &self.project_id)
    }
}

#[async_trait]
impl PushProvider for FcmProvider {
    async fn send(&self, device_token: &str, notification: &PushNotification) -> PushResult<()> {
        let body = json!({
            "message": {
                "token": device_token,
                "notification": {
                    "title": notification.title,
                    "body": notification.body,
                },
                "android": {
                    "notification": {
                        "sound": notification.sound,
                        "collapse_key": notification.collapse_key,
                    }
                },
                "data": notification.data,
            }
        });

        let resp = self
            .client
            .post(self.endpoint())
            .bearer_auth(&self.server_key)
            .json(&body)
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
            } else {
                Err(PushError::Provider(format!("FCM error {status}: {text}")))
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
                warn!(token = %token, error = %e, "FCM send failed");
            }
            results.push((token.clone(), result));
        }
        results
    }

    fn platform(&self) -> &str {
        "fcm"
    }
}
