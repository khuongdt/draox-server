use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::debug;
use crate::manager::PresenceManager;
use crate::status::PresenceStatus;

/// Background task that auto-transitions online users to Away after `idle_secs` of inactivity.
pub async fn run_auto_away(manager: Arc<PresenceManager>, idle_secs: u64, check_interval_secs: u64) {
    let mut interval = time::interval(Duration::from_secs(check_interval_secs));
    loop {
        interval.tick().await;
        let idle_threshold = chrono::Duration::seconds(idle_secs as i64);
        let now = chrono::Utc::now();

        let candidates: Vec<server_core::ClientId> = manager
            .all_presences()
            .into_iter()
            .filter(|p| {
                p.status == PresenceStatus::Online
                    && (now - p.last_activity_at) >= idle_threshold
            })
            .map(|p| p.client_id.clone())
            .collect();

        for client_id in candidates {
            debug!(client_id = %client_id, "auto-away triggered");
            manager.set_status(&client_id, PresenceStatus::Away);
        }
    }
}
