use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{error, info, warn};
use crate::manager::SecretsManager;

#[derive(Debug, Clone)]
pub struct RotationPolicy {
    pub secret_name: String,
    pub rotate_every_secs: u64,
    pub notify_on_rotate: bool,
}

/// Background rotation task that auto-rotates secrets on a schedule.
pub struct AutoRotator {
    policies: Arc<DashMap<String, RotationPolicy>>,
    manager: Arc<SecretsManager>,
}

impl AutoRotator {
    pub fn new(manager: Arc<SecretsManager>) -> Self {
        Self {
            policies: Arc::new(DashMap::new()),
            manager,
        }
    }

    pub fn add_policy(&self, policy: RotationPolicy) {
        info!(secret = %policy.secret_name, interval_secs = policy.rotate_every_secs, "rotation policy added");
        self.policies.insert(policy.secret_name.clone(), policy);
    }

    pub fn remove_policy(&self, secret_name: &str) {
        self.policies.remove(secret_name);
    }

    /// Start the rotation background loop. Checks every `check_interval_secs`.
    pub async fn run(self: Arc<Self>, check_interval_secs: u64) {
        let mut interval = time::interval(Duration::from_secs(check_interval_secs));
        let mut last_rotated: DashMap<String, chrono::DateTime<chrono::Utc>> = DashMap::new();
        info!("secret auto-rotator started");

        loop {
            interval.tick().await;
            let now = chrono::Utc::now();

            let due: Vec<RotationPolicy> = self
                .policies
                .iter()
                .filter(|p| {
                    let elapsed = last_rotated
                        .get(p.key())
                        .map(|t| (now - *t).num_seconds() as u64)
                        .unwrap_or(u64::MAX);
                    elapsed >= p.rotate_every_secs
                })
                .map(|p| p.value().clone())
                .collect();

            for policy in due {
                info!(secret = %policy.secret_name, "rotating secret");
                match self.manager.rotate(&policy.secret_name).await {
                    Ok(new_val) => {
                        info!(secret = %policy.secret_name, version = ?new_val.version, "secret rotated");
                        last_rotated.insert(policy.secret_name.clone(), now);
                    }
                    Err(e) => {
                        error!(secret = %policy.secret_name, error = %e, "secret rotation failed");
                    }
                }
            }
        }
    }
}
