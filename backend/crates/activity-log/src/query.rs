use chrono::{DateTime, Utc};

/// Filter criteria for querying log entries.
#[derive(Debug, Clone, Default)]
pub struct LogFilter {
    /// Filter by category (e.g., "connection", "session", "guard", "plugin", "server").
    pub category: Option<String>,
    /// Filter by event type (e.g., "accepted", "closed", "blocked").
    pub event_type: Option<String>,
    /// Only include entries at or after this timestamp.
    pub from: Option<DateTime<Utc>>,
    /// Only include entries at or before this timestamp.
    pub to: Option<DateTime<Utc>>,
    /// Maximum number of entries to return.
    pub limit: Option<usize>,
}
