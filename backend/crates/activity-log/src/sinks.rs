use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::RwLock;

// ────────────────────────────────────────────────────────
// SinkEntry
// ────────────────────────────────────────────────────────

/// A log entry destined for one or more [`LogSink`] implementations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkEntry {
    pub timestamp: String,
    pub level: String,
    pub category: String,
    pub message: String,
}

// ────────────────────────────────────────────────────────
// LogSink trait
// ────────────────────────────────────────────────────────

/// Trait for log sinks that receive [`SinkEntry`] records.
pub trait LogSink: Send + Sync {
    /// Write a single entry to this sink.
    fn write(&self, entry: SinkEntry);

    /// Flush any buffered data (no-op for in-memory sinks).
    fn flush(&self);

    /// Human-readable name of this sink.
    fn name(&self) -> &str;
}

// ────────────────────────────────────────────────────────
// MemorySink
// ────────────────────────────────────────────────────────

/// In-memory ring-buffer sink that keeps the most recent `max_entries`.
pub struct MemorySink {
    entries: RwLock<VecDeque<SinkEntry>>,
    max_entries: usize,
    name: String,
}

impl MemorySink {
    /// Create a new memory sink with the given name and capacity.
    pub fn new(name: String, max_entries: usize) -> Self {
        Self {
            entries: RwLock::new(VecDeque::with_capacity(max_entries.min(1024))),
            max_entries,
            name,
        }
    }

    /// Return a clone of all stored entries.
    pub fn entries(&self) -> Vec<SinkEntry> {
        let entries = self.entries.read().unwrap();
        entries.iter().cloned().collect()
    }

    /// Return the last `n` entries (or fewer if not enough exist).
    pub fn last_n(&self, n: usize) -> Vec<SinkEntry> {
        let entries = self.entries.read().unwrap();
        let start = entries.len().saturating_sub(n);
        entries.iter().skip(start).cloned().collect()
    }

    /// Remove all stored entries.
    pub fn clear(&self) {
        let mut entries = self.entries.write().unwrap();
        entries.clear();
    }

    /// Current number of stored entries.
    pub fn len(&self) -> usize {
        self.entries.read().unwrap().len()
    }

    /// Returns `true` when the sink contains no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.read().unwrap().is_empty()
    }
}

impl LogSink for MemorySink {
    fn write(&self, entry: SinkEntry) {
        let mut entries = self.entries.write().unwrap();
        entries.push_back(entry);
        while entries.len() > self.max_entries {
            entries.pop_front();
        }
    }

    fn flush(&self) {
        // No-op for in-memory sink.
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// ────────────────────────────────────────────────────────
// CompositeSink
// ────────────────────────────────────────────────────────

/// A composite sink that fans out writes to multiple child sinks.
pub struct CompositeSink {
    sinks: Vec<Box<dyn LogSink>>,
}

impl CompositeSink {
    /// Create an empty composite sink.
    pub fn new() -> Self {
        Self { sinks: Vec::new() }
    }

    /// Add a child sink.
    pub fn add_sink(&mut self, sink: Box<dyn LogSink>) {
        self.sinks.push(sink);
    }

    /// Number of child sinks.
    pub fn sink_count(&self) -> usize {
        self.sinks.len()
    }
}

impl Default for CompositeSink {
    fn default() -> Self {
        Self::new()
    }
}

impl LogSink for CompositeSink {
    fn write(&self, entry: SinkEntry) {
        for sink in &self.sinks {
            sink.write(entry.clone());
        }
    }

    fn flush(&self) {
        for sink in &self.sinks {
            sink.flush();
        }
    }

    fn name(&self) -> &str {
        "composite"
    }
}

// ────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(msg: &str) -> SinkEntry {
        SinkEntry {
            timestamp: "2026-04-14T00:00:00Z".to_string(),
            level: "INFO".to_string(),
            category: "test".to_string(),
            message: msg.to_string(),
        }
    }

    #[test]
    fn test_memory_sink_write_and_read() {
        let sink = MemorySink::new("test-sink".to_string(), 100);
        assert!(sink.is_empty());

        sink.write(make_entry("hello"));
        sink.write(make_entry("world"));

        assert_eq!(sink.len(), 2);
        assert!(!sink.is_empty());

        let entries = sink.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].message, "hello");
        assert_eq!(entries[1].message, "world");
    }

    #[test]
    fn test_memory_sink_ring_buffer_eviction() {
        let sink = MemorySink::new("ring".to_string(), 3);

        for i in 0..5 {
            sink.write(make_entry(&format!("msg-{i}")));
        }

        // Only the last 3 should remain.
        assert_eq!(sink.len(), 3);
        let entries = sink.entries();
        assert_eq!(entries[0].message, "msg-2");
        assert_eq!(entries[1].message, "msg-3");
        assert_eq!(entries[2].message, "msg-4");
    }

    #[test]
    fn test_memory_sink_last_n() {
        let sink = MemorySink::new("last-n".to_string(), 100);

        for i in 0..10 {
            sink.write(make_entry(&format!("entry-{i}")));
        }

        let last3 = sink.last_n(3);
        assert_eq!(last3.len(), 3);
        assert_eq!(last3[0].message, "entry-7");
        assert_eq!(last3[1].message, "entry-8");
        assert_eq!(last3[2].message, "entry-9");

        // Requesting more than available returns all.
        let all = sink.last_n(100);
        assert_eq!(all.len(), 10);
    }

    #[test]
    fn test_composite_sink_writes_to_all() {
        let sink_a = MemorySink::new("a".to_string(), 100);
        let sink_b = MemorySink::new("b".to_string(), 100);

        // We need shared references to verify after composite write.
        // Use Arc so the MemorySinks survive being boxed.
        use std::sync::Arc;

        let a = Arc::new(sink_a);
        let b = Arc::new(sink_b);

        // Create wrapper that delegates to Arc<MemorySink>.
        struct ArcSink(Arc<MemorySink>);
        impl LogSink for ArcSink {
            fn write(&self, entry: SinkEntry) {
                self.0.write(entry);
            }
            fn flush(&self) {
                self.0.flush();
            }
            fn name(&self) -> &str {
                self.0.name()
            }
        }

        let mut composite = CompositeSink::new();
        composite.add_sink(Box::new(ArcSink(Arc::clone(&a))));
        composite.add_sink(Box::new(ArcSink(Arc::clone(&b))));
        assert_eq!(composite.sink_count(), 2);
        assert_eq!(composite.name(), "composite");

        composite.write(make_entry("broadcast"));

        assert_eq!(a.len(), 1);
        assert_eq!(b.len(), 1);
        assert_eq!(a.entries()[0].message, "broadcast");
        assert_eq!(b.entries()[0].message, "broadcast");
    }

    #[test]
    fn test_sink_entry_fields() {
        let entry = make_entry("test-message");
        assert_eq!(entry.timestamp, "2026-04-14T00:00:00Z");
        assert_eq!(entry.level, "INFO");
        assert_eq!(entry.category, "test");
        assert_eq!(entry.message, "test-message");

        // Verify Serialize/Deserialize round-trip.
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: SinkEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.message, "test-message");
        assert_eq!(deserialized.level, "INFO");
    }

    #[test]
    fn test_memory_sink_clear() {
        let sink = MemorySink::new("clearable".to_string(), 100);
        sink.write(make_entry("a"));
        sink.write(make_entry("b"));
        assert_eq!(sink.len(), 2);

        sink.clear();
        assert!(sink.is_empty());
        assert_eq!(sink.len(), 0);
    }
}
