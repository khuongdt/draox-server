use server_core::ClientId;
use std::sync::Arc;
use tracing::{info, warn};
use crate::preferences::PreferencesStore;
use crate::provider::{PushNotification, PushProvider};
use crate::registry::{DevicePlatform, DeviceToken, DeviceTokenRegistry};

pub struct PushManager {
    registry: Arc<DeviceTokenRegistry>,
    preferences: Arc<PreferencesStore>,
    fcm_provider: Option<Arc<dyn PushProvider>>,
    apns_provider: Option<Arc<dyn PushProvider>>,
}

impl PushManager {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(DeviceTokenRegistry::new()),
            preferences: Arc::new(PreferencesStore::new()),
            fcm_provider: None,
            apns_provider: None,
        }
    }

    pub fn with_fcm(mut self, provider: Arc<dyn PushProvider>) -> Self {
        self.fcm_provider = Some(provider);
        self
    }

    pub fn with_apns(mut self, provider: Arc<dyn PushProvider>) -> Self {
        self.apns_provider = Some(provider);
        self
    }

    pub fn register_device(&self, token: DeviceToken) {
        info!(
            client_id = %token.client_id,
            platform = ?token.platform,
            "device token registered"
        );
        self.registry.register(token);
    }

    pub fn unregister_device(&self, client_id: &ClientId, token: &str) {
        self.registry.unregister(client_id, token);
    }

    /// Send a push notification to all devices for a client.
    pub async fn send_to_client(
        &self,
        client_id: &ClientId,
        notification: &PushNotification,
        topic: Option<&str>,
    ) {
        let prefs = self.preferences.get_or_default(client_id);
        if !prefs.should_notify(topic) {
            return;
        }

        let tokens = self.registry.get_tokens(client_id);
        for token in tokens {
            let provider = match &token.platform {
                DevicePlatform::Fcm => self.fcm_provider.as_ref(),
                DevicePlatform::Apns => self.apns_provider.as_ref(),
            };
            if let Some(p) = provider {
                match p.send(&token.token, notification).await {
                    Ok(_) => {
                        self.registry.mark_used(client_id, &token.token);
                        self.preferences.increment_badge(client_id);
                    }
                    Err(e) => warn!(
                        client_id = %client_id,
                        token = %token.token,
                        error = %e,
                        "push send failed"
                    ),
                }
            }
        }
    }

    /// Broadcast to many clients.
    pub async fn send_to_clients(
        &self,
        client_ids: &[ClientId],
        notification: &PushNotification,
        topic: Option<&str>,
    ) {
        for id in client_ids {
            self.send_to_client(id, notification, topic).await;
        }
    }

    pub fn registry(&self) -> &DeviceTokenRegistry {
        &self.registry
    }

    pub fn preferences(&self) -> &PreferencesStore {
        &self.preferences
    }
}

impl Default for PushManager {
    fn default() -> Self {
        Self::new()
    }
}
