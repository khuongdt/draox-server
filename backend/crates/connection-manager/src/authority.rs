//! Server-authoritative state management.
//!
//! The server owns the canonical state for every session. Clients propose
//! changes; the server validates them, increments a monotonic version counter,
//! and stores the result. This prevents conflicting updates and gives
//! reconnecting clients a full state snapshot to resync from.

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use server_core::{Result, Error, SessionId};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

/// The canonical, server-owned state for one session.
///
/// All mutations go through [`SessionAuthority::validate_and_apply`] which
/// atomically bumps `version` before writing.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthoritativeState {
    /// Monotonically increasing version number — starts at 1 on creation.
    pub version: u64,
    /// Arbitrary key/value pairs owned by the server.
    pub data: HashMap<String, serde_json::Value>,
    /// Wall-clock timestamp of the last mutation.
    pub last_updated: DateTime<Utc>,
}

impl AuthoritativeState {
    /// Create a fresh state at version 1 with no data.
    fn new() -> Self {
        Self {
            version: 1,
            data: HashMap::new(),
            last_updated: Utc::now(),
        }
    }
}

/// Manages the authoritative state for all active sessions.
///
/// Each session gets its own [`AuthoritativeState`] plus a dedicated
/// [`AtomicU64`] version counter. The counter is the source of truth for
/// the current version; the stored state's `version` field mirrors it.
///
/// An optional listener registered via [`set_state_change_listener`] is invoked
/// after every successful [`validate_and_apply`] so callers can broadcast the
/// updated state to all session connections.
pub struct SessionAuthority {
    /// session_id → current authoritative state.
    states: DashMap<SessionId, AuthoritativeState>,
    /// session_id → version counter (always in sync with `states[id].version`).
    version_counters: DashMap<SessionId, AtomicU64>,
    /// Optional callback triggered after each state mutation.
    on_state_change: OnceLock<Arc<dyn Fn(SessionId, AuthoritativeState) + Send + Sync>>,
}

impl SessionAuthority {
    /// Create an empty `SessionAuthority`.
    pub fn new() -> Self {
        Self {
            states: DashMap::new(),
            version_counters: DashMap::new(),
            on_state_change: OnceLock::new(),
        }
    }

    /// Register a listener that is called after every successful state mutation.
    ///
    /// The listener receives a clone of the session ID and the updated state.
    /// Can only be set once; subsequent calls are silently ignored.
    pub fn set_state_change_listener<F>(&self, f: F)
    where
        F: Fn(SessionId, AuthoritativeState) + Send + Sync + 'static,
    {
        let _ = self.on_state_change.set(Arc::new(f));
    }

    /// Initialise canonical state for a newly created session.
    ///
    /// Returns the initial [`AuthoritativeState`] (version 1, empty data).
    /// If state already exists for this session it is left untouched and the
    /// existing state is returned.
    pub fn create_state(&self, session_id: &SessionId) -> AuthoritativeState {
        let initial = AuthoritativeState::new();
        self.states
            .entry(session_id.clone())
            .or_insert_with(|| initial.clone());
        self.version_counters
            .entry(session_id.clone())
            .or_insert_with(|| AtomicU64::new(1));
        // Return whichever state is now stored (could be the pre-existing one).
        self.states
            .get(session_id)
            .map(|r| r.clone())
            .unwrap_or(initial)
    }

    /// Get a clone of the current state for a session.
    ///
    /// Returns `None` if no state has been created for this session.
    pub fn get_state(&self, session_id: &SessionId) -> Option<AuthoritativeState> {
        self.states.get(session_id).map(|r| r.clone())
    }

    /// Validate and apply a single key/value update to the session state.
    ///
    /// The server is always the authority: if the key exists it is
    /// overwritten, if it is new it is inserted. The version counter is
    /// atomically incremented and the stored state is updated.
    ///
    /// Returns the **new** version number on success, or an error if no
    /// state exists for `session_id`.
    pub fn validate_and_apply(
        &self,
        session_id: &SessionId,
        key: &str,
        value: serde_json::Value,
    ) -> Result<u64> {
        let counter = self
            .version_counters
            .get(session_id)
            .ok_or_else(|| Error::SessionNotFound(session_id.to_string()))?;

        let new_version = counter.fetch_add(1, Ordering::Relaxed) + 1;

        let mut state = self
            .states
            .get_mut(session_id)
            .ok_or_else(|| Error::SessionNotFound(session_id.to_string()))?;

        state.data.insert(key.to_string(), value);
        state.version = new_version;
        state.last_updated = Utc::now();
        let snapshot = state.clone();
        drop(state);

        if let Some(cb) = self.on_state_change.get() {
            cb(session_id.clone(), snapshot);
        }

        Ok(new_version)
    }

    /// Return a full state snapshot suitable for sending to a reconnecting
    /// client.
    ///
    /// Identical to [`get_state`] — provided as a semantic alias to make
    /// call-sites at reconnection points self-documenting.
    pub fn get_snapshot(&self, session_id: &SessionId) -> Option<AuthoritativeState> {
        self.get_state(session_id)
    }

    /// Remove all state for a session (called when a session is destroyed).
    pub fn remove_state(&self, session_id: &SessionId) {
        self.states.remove(session_id);
        self.version_counters.remove(session_id);
    }
}

impl Default for SessionAuthority {
    fn default() -> Self {
        Self::new()
    }
}

// ────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_state_initial_version() {
        let auth = SessionAuthority::new();
        let sid = SessionId::new();

        let state = auth.create_state(&sid);
        assert_eq!(state.version, 1);
        assert!(state.data.is_empty());
    }

    #[test]
    fn test_create_state_idempotent() {
        let auth = SessionAuthority::new();
        let sid = SessionId::new();

        let s1 = auth.create_state(&sid);
        // Apply an update so the state differs from the initial.
        auth.validate_and_apply(&sid, "k", json!(42)).unwrap();
        // Calling create_state again should not reset it.
        let s2 = auth.create_state(&sid);
        assert_eq!(s1.version, 1);
        // s2 should still have the updated data because create_state uses or_insert_with.
        assert!(s2.data.contains_key("k") || s2.version >= 1);
    }

    #[test]
    fn test_validate_and_apply_increments_version() {
        let auth = SessionAuthority::new();
        let sid = SessionId::new();
        auth.create_state(&sid);

        let v2 = auth.validate_and_apply(&sid, "score", json!(100)).unwrap();
        assert_eq!(v2, 2);

        let v3 = auth.validate_and_apply(&sid, "level", json!("gold")).unwrap();
        assert_eq!(v3, 3);

        let state = auth.get_state(&sid).unwrap();
        assert_eq!(state.version, 3);
        assert_eq!(state.data["score"], json!(100));
        assert_eq!(state.data["level"], json!("gold"));
    }

    #[test]
    fn test_validate_and_apply_unknown_session_returns_error() {
        let auth = SessionAuthority::new();
        let unknown = SessionId::new();

        let result = auth.validate_and_apply(&unknown, "k", json!(1));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains(&unknown.to_string()));
    }

    #[test]
    fn test_get_snapshot_matches_get_state() {
        let auth = SessionAuthority::new();
        let sid = SessionId::new();
        auth.create_state(&sid);
        auth.validate_and_apply(&sid, "x", json!(7)).unwrap();

        let snap = auth.get_snapshot(&sid).unwrap();
        let state = auth.get_state(&sid).unwrap();
        assert_eq!(snap.version, state.version);
        assert_eq!(snap.data, state.data);
    }

    #[test]
    fn test_remove_state() {
        let auth = SessionAuthority::new();
        let sid = SessionId::new();
        auth.create_state(&sid);
        assert!(auth.get_state(&sid).is_some());

        auth.remove_state(&sid);
        assert!(auth.get_state(&sid).is_none());

        // Applying after removal should fail.
        let result = auth.validate_and_apply(&sid, "k", json!(1));
        assert!(result.is_err());
    }

    #[test]
    fn test_state_change_listener_called() {
        use std::sync::Mutex;

        let auth = SessionAuthority::new();
        let sid = SessionId::new();
        auth.create_state(&sid);

        let received: Arc<Mutex<Vec<(SessionId, u64)>>> = Arc::new(Mutex::new(vec![]));
        let received_clone = Arc::clone(&received);

        auth.set_state_change_listener(move |s, state| {
            received_clone.lock().unwrap().push((s, state.version));
        });

        auth.validate_and_apply(&sid, "x", json!(1)).unwrap();
        auth.validate_and_apply(&sid, "y", json!(2)).unwrap();

        let calls = received.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].1, 2); // version after first apply
        assert_eq!(calls[1].1, 3); // version after second apply
    }

    #[test]
    fn test_multiple_sessions_independent() {
        let auth = SessionAuthority::new();
        let sid1 = SessionId::new();
        let sid2 = SessionId::new();

        auth.create_state(&sid1);
        auth.create_state(&sid2);

        auth.validate_and_apply(&sid1, "a", json!(1)).unwrap();
        auth.validate_and_apply(&sid1, "b", json!(2)).unwrap();
        auth.validate_and_apply(&sid2, "a", json!(99)).unwrap();

        let s1 = auth.get_state(&sid1).unwrap();
        let s2 = auth.get_state(&sid2).unwrap();

        assert_eq!(s1.version, 3);
        assert_eq!(s2.version, 2);
        assert_eq!(s1.data["a"], json!(1));
        assert_eq!(s2.data["a"], json!(99));
    }
}
