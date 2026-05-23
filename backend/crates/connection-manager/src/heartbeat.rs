use crate::manager::SessionManager;
use server_config::model::SessionConfig;
use server_core::ShutdownReceiver;
use std::sync::Arc;
use tokio::time::{self, Duration};
use tracing::{debug, info};

/// Background task that periodically cleans up expired sessions.
///
/// A session is considered expired when it has no connections and
/// `last_activity + grace_period_secs` has elapsed.
///
/// The task runs every 10 seconds (or `heartbeat_interval_secs` from config)
/// and shuts down gracefully when the shutdown signal is received.
pub async fn session_cleanup_task(
    manager: Arc<SessionManager>,
    config: SessionConfig,
    mut shutdown: ShutdownReceiver,
) {
    // Use a fixed 10-second interval for cleanup checks
    let cleanup_interval = Duration::from_secs(10);
    let mut interval = time::interval(cleanup_interval);
    // Skip the first immediate tick
    interval.tick().await;

    info!(
        interval_secs = 10,
        grace_period_secs = config.grace_period_secs,
        "session cleanup task started"
    );

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let expired = manager.expired_empty_sessions(config.grace_period_secs);
                if !expired.is_empty() {
                    debug!(count = expired.len(), "cleaning up expired sessions");
                }
                for session_id in expired {
                    manager.destroy_session(&session_id, "grace period expired");
                }
            }
            _ = shutdown.recv() => {
                info!("session cleanup task shutting down");
                break;
            }
        }
    }
}
