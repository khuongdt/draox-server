use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum PushError {
    #[error("invalid device token")]
    InvalidToken,
    #[error("provider error: {0}")]
    Provider(String),
    #[error("rate limited")]
    RateLimited,
}

pub type PushResult<T> = Result<T, PushError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushNotification {
    pub title: String,
    pub body: String,
    pub badge: Option<u32>,
    pub sound: Option<String>,
    pub data: std::collections::HashMap<String, String>,
    pub topic: Option<String>,
    pub collapse_key: Option<String>,
}

impl PushNotification {
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            badge: None,
            sound: Some("default".to_string()),
            data: Default::default(),
            topic: None,
            collapse_key: None,
        }
    }
}

#[async_trait]
pub trait PushProvider: Send + Sync + 'static {
    async fn send(&self, device_token: &str, notification: &PushNotification) -> PushResult<()>;
    async fn send_batch(
        &self,
        tokens: &[String],
        notification: &PushNotification,
    ) -> Vec<(String, PushResult<()>)>;
    fn platform(&self) -> &str;
}
