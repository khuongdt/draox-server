//! Server-Sent Events (SSE) support.
//!
//! `SseEvent` models a single SSE message.  `SseStream` is a thin wrapper
//! around a channel sender that the HTTP handler can write to.  `SseManager`
//! holds named channels (topics) and lets the application broadcast events to
//! all subscribers on a topic.

use dashmap::DashMap;
use tokio::sync::mpsc;

// ─── Event ────────────────────────────────────────────────────────────────────

/// A single Server-Sent Event.
#[derive(Debug, Clone)]
pub struct SseEvent {
    /// Optional event type (maps to `event:` field).
    pub event: Option<String>,
    /// Payload (maps to `data:` field).  Required.
    pub data: String,
    /// Optional last-event ID (maps to `id:` field).
    pub id: Option<String>,
    /// Optional reconnection hint in milliseconds (maps to `retry:` field).
    pub retry: Option<u64>,
}

impl SseEvent {
    /// Construct a plain data event.
    pub fn new(data: impl Into<String>) -> Self {
        Self {
            event: None,
            data: data.into(),
            id: None,
            retry: None,
        }
    }

    /// Set the event type.
    pub fn with_event(mut self, event: impl Into<String>) -> Self {
        self.event = Some(event.into());
        self
    }

    /// Set the last-event ID.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the client reconnect hint (milliseconds).
    pub fn with_retry(mut self, retry_ms: u64) -> Self {
        self.retry = Some(retry_ms);
        self
    }

    /// Serialise the event into the SSE wire format.
    ///
    /// ```text
    /// id: 42\n
    /// event: update\n
    /// data: hello world\n
    /// retry: 3000\n
    /// \n
    /// ```
    pub fn format(&self) -> String {
        let mut buf = String::new();

        if let Some(ref id) = self.id {
            buf.push_str("id: ");
            buf.push_str(id);
            buf.push('\n');
        }
        if let Some(ref event) = self.event {
            buf.push_str("event: ");
            buf.push_str(event);
            buf.push('\n');
        }
        // Multi-line data: each physical line gets its own `data:` prefix.
        for line in self.data.lines() {
            buf.push_str("data: ");
            buf.push_str(line);
            buf.push('\n');
        }
        if self.data.is_empty() {
            buf.push_str("data: \n");
        }
        if let Some(ms) = self.retry {
            buf.push_str(&format!("retry: {ms}\n"));
        }
        buf.push('\n'); // blank line terminates the event
        buf
    }
}

// ─── Stream ───────────────────────────────────────────────────────────────────

/// Handle to a single SSE subscriber.
///
/// Wraps an `mpsc::Sender`; drop it to signal that the subscriber is gone.
#[derive(Clone, Debug)]
pub struct SseStream {
    pub(crate) sender: mpsc::Sender<SseEvent>,
}

impl SseStream {
    /// Send an event to this subscriber.  Returns `false` if the receiver has
    /// been dropped (subscriber disconnected).
    pub async fn send(&self, event: SseEvent) -> bool {
        self.sender.send(event).await.is_ok()
    }
}

// ─── Manager ─────────────────────────────────────────────────────────────────

/// Manages named SSE channels (topics).
///
/// Multiple subscribers can listen on the same channel.  `broadcast` fans
/// events out to every active subscriber; `remove_closed` prunes senders whose
/// receivers have been dropped.
pub struct SseManager {
    /// channel name → list of active sender halves
    streams: DashMap<String, Vec<mpsc::Sender<SseEvent>>>,
}

impl SseManager {
    pub fn new() -> Self {
        Self {
            streams: DashMap::new(),
        }
    }

    /// Subscribe to `channel`.  Returns the `Receiver` half that the HTTP
    /// response handler should read from and stream to the client.
    ///
    /// The channel capacity is 64 events; back-pressure is applied when the
    /// client is slow.
    pub fn subscribe(&self, channel: &str) -> mpsc::Receiver<SseEvent> {
        let (tx, rx) = mpsc::channel(64);
        self.streams
            .entry(channel.to_string())
            .or_default()
            .push(tx);
        rx
    }

    /// Broadcast `event` to every active subscriber on `channel`.
    ///
    /// Senders whose receivers have been dropped are silently ignored (they
    /// will be cleaned up by the next call to `remove_closed`).
    pub fn broadcast(&self, channel: &str, event: SseEvent) {
        if let Some(senders) = self.streams.get(channel) {
            for tx in senders.iter() {
                // Non-blocking; if the channel is full the event is dropped
                // for that subscriber — keep the others going.
                let _ = tx.try_send(event.clone());
            }
        }
    }

    /// Remove senders whose receiver has been dropped (subscriber gone).
    ///
    /// Call this periodically (e.g. after each broadcast) to avoid memory
    /// growth.
    pub fn remove_closed(&self, channel: &str) {
        if let Some(mut senders) = self.streams.get_mut(channel) {
            senders.retain(|tx| !tx.is_closed());
        }
    }
}

impl Default for SseManager {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_event_format_data_only() {
        let event = SseEvent::new("hello");
        let formatted = event.format();
        assert!(formatted.contains("data: hello\n"));
        assert!(formatted.ends_with("\n\n"));
    }

    #[test]
    fn test_sse_event_format_all_fields() {
        let event = SseEvent::new("payload")
            .with_event("update")
            .with_id("42")
            .with_retry(3000);
        let formatted = event.format();
        assert!(formatted.contains("id: 42\n"));
        assert!(formatted.contains("event: update\n"));
        assert!(formatted.contains("data: payload\n"));
        assert!(formatted.contains("retry: 3000\n"));
        assert!(formatted.ends_with("\n\n"));
    }

    #[test]
    fn test_sse_event_multiline_data() {
        let event = SseEvent::new("line1\nline2");
        let formatted = event.format();
        assert!(formatted.contains("data: line1\n"));
        assert!(formatted.contains("data: line2\n"));
    }

    #[tokio::test]
    async fn test_sse_manager_subscribe_and_broadcast() {
        let manager = SseManager::new();
        let mut rx = manager.subscribe("news");

        manager.broadcast("news", SseEvent::new("breaking"));

        let received = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            rx.recv(),
        )
        .await
        .expect("timeout")
        .expect("channel closed");

        assert_eq!(received.data, "breaking");
    }

    #[tokio::test]
    async fn test_sse_manager_remove_closed() {
        let manager = SseManager::new();
        let rx = manager.subscribe("topic");
        // Drop the receiver — the sender is now closed.
        drop(rx);

        manager.remove_closed("topic");

        // After cleanup the senders list should be empty.
        let senders = manager.streams.get("topic").map(|s| s.len()).unwrap_or(0);
        assert_eq!(senders, 0);
    }
}
