use async_graphql::{Context, Result, SimpleObject, Subscription};
use futures_util::Stream;
use std::time::Duration;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;

/// A connection event pushed over the subscription.
#[derive(SimpleObject, Clone)]
pub struct ConnectionEvent {
    pub event_type: String,
    pub client_id: String,
    pub timestamp: String,
}

pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    /// Stream connection events in real-time.
    /// Currently yields a heartbeat tick; wire to connection-manager EventBus for real events.
    async fn connection_events(
        &self,
        _ctx: &Context<'_>,
    ) -> Result<impl Stream<Item = ConnectionEvent>> {
        let stream = IntervalStream::new(tokio::time::interval(Duration::from_secs(30)))
            .map(|_| ConnectionEvent {
                event_type: "heartbeat".into(),
                client_id: "server".into(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        Ok(stream)
    }
}
