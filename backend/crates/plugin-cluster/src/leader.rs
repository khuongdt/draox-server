use fred::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{info, warn};
use crate::node::NodeId;

const LEADER_KEY: &str = "draox:cluster:leader";
const LEADER_TTL_SECS: u64 = 10;
const HEARTBEAT_INTERVAL_SECS: u64 = 5;

/// Distributed leader election via Redis SETNX + TTL.
///
/// The leader periodically refreshes its lock. Other nodes attempt to
/// acquire the lock when the current leader's TTL expires.
pub struct LeaderElection {
    redis: Arc<Client>,
    node_id: NodeId,
    is_leader: Arc<std::sync::atomic::AtomicBool>,
}

impl LeaderElection {
    pub async fn new(redis_url: &str, node_id: NodeId) -> anyhow::Result<Self> {
        let config = Config::from_url(redis_url)?;
        let client = Arc::new(Client::new(config, None, None, None));
        client.connect();
        client.wait_for_connect().await?;
        Ok(Self {
            redis: client,
            node_id,
            is_leader: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    /// Try to become the leader. Returns true if this node is now leader.
    pub async fn try_acquire(&self) -> anyhow::Result<bool> {
        // SET key value NX EX ttl — atomically set only if key does not exist
        let result: Value = self
            .redis
            .set(
                LEADER_KEY,
                self.node_id.as_str(),
                Some(Expiration::EX(LEADER_TTL_SECS as i64)),
                Some(SetOptions::NX),
                false,
            )
            .await?;
        let acquired = !result.is_null();
        if acquired {
            self.is_leader.store(true, std::sync::atomic::Ordering::SeqCst);
            info!(node = %self.node_id, "acquired cluster leadership");
        }
        Ok(acquired)
    }

    /// Check if we still hold the leader lock.
    pub async fn is_still_leader(&self) -> anyhow::Result<bool> {
        let current: Option<String> = self.redis.get(LEADER_KEY).await?;
        let still = current.as_deref() == Some(self.node_id.as_str());
        self.is_leader.store(still, std::sync::atomic::Ordering::SeqCst);
        Ok(still)
    }

    /// Refresh leader TTL while we are leader.
    pub async fn refresh(&self) -> anyhow::Result<bool> {
        if !self.is_leader() {
            return Ok(false);
        }
        // Only refresh if we own the key (compare-and-set via Lua script)
        let script = r#"
            if redis.call('get', KEYS[1]) == ARGV[1] then
                return redis.call('expire', KEYS[1], ARGV[2])
            else
                return 0
            end
        "#;
        let ttl_str = LEADER_TTL_SECS.to_string();
        let result: i64 = self
            .redis
            .eval(script, vec![LEADER_KEY], vec![
                self.node_id.as_str(),
                ttl_str.as_str(),
            ])
            .await?;
        let still = result == 1;
        self.is_leader.store(still, std::sync::atomic::Ordering::SeqCst);
        if !still {
            warn!(node = %self.node_id, "lost cluster leadership");
        }
        Ok(still)
    }

    /// Voluntarily release leadership.
    pub async fn release(&self) -> anyhow::Result<()> {
        let script = r#"
            if redis.call('get', KEYS[1]) == ARGV[1] then
                return redis.call('del', KEYS[1])
            else
                return 0
            end
        "#;
        self.redis
            .eval::<Value, _, _, _>(script, vec![LEADER_KEY], vec![self.node_id.as_str()])
            .await?;
        self.is_leader.store(false, std::sync::atomic::Ordering::SeqCst);
        info!(node = %self.node_id, "released cluster leadership");
        Ok(())
    }

    /// Returns true if this node currently believes it is leader.
    pub fn is_leader(&self) -> bool {
        self.is_leader.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Start a background heartbeat task that keeps the leader lock alive.
    pub fn start_heartbeat(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
            loop {
                interval.tick().await;
                if self.is_leader() {
                    if let Err(e) = self.refresh().await {
                        warn!("leader heartbeat error: {e}");
                    }
                } else if let Err(e) = self.try_acquire().await {
                    warn!("leader election attempt error: {e}");
                }
            }
        });
    }
}
