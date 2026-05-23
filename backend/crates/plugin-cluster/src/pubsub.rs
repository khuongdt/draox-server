use fred::clients::SubscriberClient;
use fred::prelude::*;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, info};
use crate::node::{ClusterMessage, NodeId};

const CLUSTER_CHANNEL_PREFIX: &str = "draox:cluster:";

/// Inter-node messaging via Redis Pub/Sub.
pub struct ClusterPubSub {
    publisher: Arc<Client>,
    node_id: NodeId,
    tx: broadcast::Sender<ClusterMessage>,
}

impl ClusterPubSub {
    pub async fn new(
        redis_url: &str,
        node_id: NodeId,
    ) -> anyhow::Result<(Self, Arc<SubscriberClient>)> {
        let config = Config::from_url(redis_url)?;
        let publisher = Arc::new(Client::new(config.clone(), None, None, None));
        publisher.connect();
        publisher.wait_for_connect().await?;

        let subscriber = Arc::new(SubscriberClient::new(config, None, None, None));
        subscriber.connect();
        subscriber.wait_for_connect().await?;

        let (tx, _) = broadcast::channel(1024);
        let tx_clone = tx.clone();
        let sub_clone = subscriber.clone();
        let node_id_clone = node_id.clone();

        tokio::spawn(async move {
            let channel = format!("{}broadcast", CLUSTER_CHANNEL_PREFIX);
            if let Err(e) = sub_clone.subscribe(&channel).await {
                error!("cluster pubsub subscribe error: {e}");
                return;
            }
            info!(node = %node_id_clone, "cluster pubsub listening");
            let mut stream = sub_clone.message_rx();
            while let Ok(msg) = stream.recv().await {
                if let Some(payload_str) = msg.value.as_str() {
                    let payload_str = payload_str.as_ref();
                    if let Ok(cluster_msg) = serde_json::from_str::<ClusterMessage>(payload_str) {
                        // Ignore own messages
                        if cluster_msg.from_node != node_id_clone {
                            let _ = tx_clone.send(cluster_msg);
                        }
                    }
                }
            }
        });

        Ok((
            Self { publisher, node_id, tx },
            subscriber,
        ))
    }

    /// Publish a message to all cluster nodes.
    pub async fn publish(&self, topic: String, payload: serde_json::Value) -> anyhow::Result<()> {
        let msg = ClusterMessage::new(self.node_id.clone(), topic, payload);
        let channel = format!("{}broadcast", CLUSTER_CHANNEL_PREFIX);
        let json = serde_json::to_string(&msg)?;
        self.publisher.publish::<Value, _, _>(&channel, json).await?;
        Ok(())
    }

    /// Subscribe to incoming cluster messages.
    pub fn subscribe(&self) -> broadcast::Receiver<ClusterMessage> {
        self.tx.subscribe()
    }

    /// Publish a heartbeat for this node.
    pub async fn heartbeat(&self, info: &crate::node::NodeInfo) -> anyhow::Result<()> {
        self.publish(
            "node.heartbeat".to_string(),
            serde_json::to_value(info)?,
        ).await
    }
}
