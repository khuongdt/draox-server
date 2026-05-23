use crate::session::{ClientSession, SessionInfo};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use server_config::model::SessionConfig;
use server_core::event::{EventBus, ServerEvent};
use server_core::{ClientId, ConnectionId, ConnectionRole, Error, Result, SessionId};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Per-session metrics tracking bytes in/out and message count.
///
/// Uses atomic counters for lock-free concurrent updates.
pub struct SessionMetrics {
    pub bytes_in: AtomicU64,
    pub bytes_out: AtomicU64,
    pub message_count: AtomicU64,
    pub connected_at: DateTime<Utc>,
}

impl SessionMetrics {
    /// Create a new SessionMetrics with all counters at zero.
    fn new() -> Self {
        Self {
            bytes_in: AtomicU64::new(0),
            bytes_out: AtomicU64::new(0),
            message_count: AtomicU64::new(0),
            connected_at: Utc::now(),
        }
    }

    /// Take a point-in-time snapshot of the metrics.
    fn snapshot(&self) -> SessionMetricsSnapshot {
        let now = Utc::now();
        SessionMetricsSnapshot {
            bytes_in: self.bytes_in.load(Ordering::Relaxed),
            bytes_out: self.bytes_out.load(Ordering::Relaxed),
            message_count: self.message_count.load(Ordering::Relaxed),
            connected_duration_secs: now
                .signed_duration_since(self.connected_at)
                .num_seconds(),
        }
    }
}

/// Serializable point-in-time snapshot of session metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetricsSnapshot {
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub message_count: u64,
    pub connected_duration_secs: i64,
}

/// Manages all active client sessions and their connection bindings.
///
/// Provides O(1) lookups by session ID, connection ID, or client ID
/// via concurrent `DashMap` indices.
pub struct SessionManager {
    /// Primary store: session_id -> ClientSession
    sessions: DashMap<SessionId, ClientSession>,
    /// Index: connection_id -> session_id
    conn_to_session: DashMap<ConnectionId, SessionId>,
    /// Index: client_id -> session_id
    client_to_session: DashMap<ClientId, SessionId>,
    /// Session configuration (max connections, timeouts, etc.)
    config: SessionConfig,
    /// Event bus for publishing session lifecycle events
    event_bus: Arc<EventBus>,
    /// Per-session metrics: bytes_in, bytes_out, message_count
    session_metrics: DashMap<SessionId, SessionMetrics>,
    /// Draining sessions (no new connections accepted)
    draining: DashMap<SessionId, bool>,
}

impl SessionManager {
    /// Create a new SessionManager with the given config and event bus.
    pub fn new(config: SessionConfig, event_bus: Arc<EventBus>) -> Self {
        Self {
            sessions: DashMap::new(),
            conn_to_session: DashMap::new(),
            client_to_session: DashMap::new(),
            config,
            event_bus,
            session_metrics: DashMap::new(),
            draining: DashMap::new(),
        }
    }

    /// Create a new session for the given client.
    ///
    /// Publishes a `SessionCreated` event on the event bus.
    pub fn create_session(&self, client_id: ClientId) -> SessionId {
        let session = ClientSession::new(client_id.clone());
        let session_id = session.session_id.clone();

        self.sessions.insert(session_id.clone(), session);
        self.client_to_session
            .insert(client_id.clone(), session_id.clone());
        self.session_metrics
            .insert(session_id.clone(), SessionMetrics::new());

        self.event_bus.publish(ServerEvent::SessionCreated {
            session_id: session_id.clone(),
        });

        info!(session_id = %session_id, client_id = %client_id, "session created");
        session_id
    }

    /// Bind a connection to an existing session with the given role.
    ///
    /// Returns an error if:
    /// - The session does not exist
    /// - Max connections per session is exceeded
    /// - Role constraints are violated (duplicate Primary/Control)
    pub fn bind_connection(
        &self,
        session_id: &SessionId,
        conn_id: ConnectionId,
        role: ConnectionRole,
    ) -> Result<()> {
        // Reject new connections if the session is draining
        if self.is_draining(session_id) {
            return Err(Error::Connection(format!(
                "session {session_id} is draining, no new connections accepted"
            )));
        }

        let mut session = self.sessions.get_mut(session_id).ok_or_else(|| {
            Error::SessionNotFound(session_id.to_string())
        })?;

        // Check max connections per session
        if session.connection_count() >= self.config.max_connections_per_session {
            return Err(Error::MaxConnectionsReached {
                max: self.config.max_connections_per_session,
            });
        }

        session.add_connection(conn_id.clone(), role)?;
        // Drop the mutable ref before inserting into the index
        let sid = session_id.clone();
        drop(session);

        self.conn_to_session.insert(conn_id.clone(), sid);

        debug!(
            session_id = %session_id,
            conn_id = %conn_id,
            role = ?role,
            "connection bound to session"
        );
        Ok(())
    }

    /// Unbind a connection from its session.
    ///
    /// Returns the session ID if the connection was bound.
    /// When the last connection is removed, the session is NOT destroyed
    /// immediately — the heartbeat cleanup task handles grace-period expiry.
    pub fn unbind_connection(&self, conn_id: &ConnectionId) -> Option<SessionId> {
        let session_id = self.conn_to_session.remove(conn_id).map(|(_, sid)| sid)?;

        if let Some(mut session) = self.sessions.get_mut(&session_id) {
            session.remove_connection(conn_id);
            let remaining = session.connection_count();
            drop(session);

            debug!(
                session_id = %session_id,
                conn_id = %conn_id,
                remaining_connections = remaining,
                "connection unbound from session"
            );

            if remaining == 0 {
                debug!(
                    session_id = %session_id,
                    grace_period_secs = self.config.grace_period_secs,
                    "session has no connections, grace period started"
                );
            }
        }

        Some(session_id)
    }

    /// Get a reference to a session by its ID.
    ///
    /// Returns a DashMap `Ref` guard that holds a read lock on the entry.
    pub fn get_session(
        &self,
        session_id: &SessionId,
    ) -> Option<dashmap::mapref::one::Ref<'_, SessionId, ClientSession>> {
        self.sessions.get(session_id)
    }

    /// Look up which session a connection belongs to.
    pub fn get_session_by_connection(&self, conn_id: &ConnectionId) -> Option<SessionId> {
        self.conn_to_session.get(conn_id).map(|r| r.value().clone())
    }

    /// Look up the session for a given client.
    pub fn get_session_by_client(&self, client_id: &ClientId) -> Option<SessionId> {
        self.client_to_session
            .get(client_id)
            .map(|r| r.value().clone())
    }

    /// Destroy a session, removing all index entries.
    ///
    /// Publishes a `SessionDestroyed` event.
    pub fn destroy_session(&self, session_id: &SessionId, reason: &str) {
        if let Some((_, session)) = self.sessions.remove(session_id) {
            // Remove all connection -> session mappings
            for (conn_id, _) in &session.connections {
                self.conn_to_session.remove(conn_id);
            }

            // Remove client -> session mapping
            self.client_to_session.remove(&session.client_id);

            // Remove metrics and draining state
            self.session_metrics.remove(session_id);
            self.draining.remove(session_id);

            self.event_bus.publish(ServerEvent::SessionDestroyed {
                session_id: session_id.clone(),
                reason: reason.to_string(),
            });

            info!(
                session_id = %session_id,
                client_id = %session.client_id,
                reason = reason,
                "session destroyed"
            );
        } else {
            warn!(session_id = %session_id, "attempted to destroy non-existent session");
        }
    }

    /// Total number of active sessions.
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Total number of bound connections across all sessions.
    pub fn connection_count(&self) -> usize {
        self.conn_to_session.len()
    }

    /// Return a summary list of all active sessions.
    pub fn sessions_list(&self) -> Vec<SessionInfo> {
        self.sessions
            .iter()
            .map(|entry| SessionInfo::from(entry.value()))
            .collect()
    }

    /// Access the session configuration.
    pub fn config(&self) -> &SessionConfig {
        &self.config
    }

    /// Iterate over all sessions, returning (SessionId, is_empty, last_activity).
    ///
    /// Used by the heartbeat cleanup task to find expired sessions.
    pub(crate) fn expired_empty_sessions(&self, grace_period_secs: u64) -> Vec<SessionId> {
        let now = chrono::Utc::now();
        self.sessions
            .iter()
            .filter_map(|entry| {
                let session = entry.value();
                if session.is_empty() {
                    let elapsed = now
                        .signed_duration_since(session.last_activity)
                        .num_seconds();
                    if elapsed >= grace_period_secs as i64 {
                        return Some(session.session_id.clone());
                    }
                }
                None
            })
            .collect()
    }

    /// Promote a connection's role within its session.
    ///
    /// Finds the session via `conn_to_session` index, then delegates
    /// to `ClientSession::promote_connection`.
    pub fn promote_connection(
        &self,
        conn_id: &ConnectionId,
        new_role: ConnectionRole,
    ) -> Result<()> {
        let session_id = self
            .conn_to_session
            .get(conn_id)
            .map(|r| r.value().clone())
            .ok_or_else(|| {
                Error::Connection(format!("connection {conn_id} not bound to any session"))
            })?;

        let mut session = self.sessions.get_mut(&session_id).ok_or_else(|| {
            Error::SessionNotFound(session_id.to_string())
        })?;

        session.promote_connection(conn_id, new_role)?;
        debug!(
            session_id = %session_id,
            conn_id = %conn_id,
            new_role = ?new_role,
            "connection promoted"
        );
        Ok(())
    }

    /// Demote a connection to Streaming role within its session.
    ///
    /// Finds the session via `conn_to_session` index, then delegates
    /// to `ClientSession::demote_connection`.
    pub fn demote_connection(&self, conn_id: &ConnectionId) -> Result<()> {
        let session_id = self
            .conn_to_session
            .get(conn_id)
            .map(|r| r.value().clone())
            .ok_or_else(|| {
                Error::Connection(format!("connection {conn_id} not bound to any session"))
            })?;

        let mut session = self.sessions.get_mut(&session_id).ok_or_else(|| {
            Error::SessionNotFound(session_id.to_string())
        })?;

        session.demote_connection(conn_id)?;
        debug!(
            session_id = %session_id,
            conn_id = %conn_id,
            "connection demoted to streaming"
        );
        Ok(())
    }

    /// Migrate a connection from one session to another.
    ///
    /// Unbinds the connection from its source session and binds it to the
    /// target session with the given role. The target session must exist
    /// and must not be draining.
    pub fn migrate_connection(
        &self,
        conn_id: &ConnectionId,
        target_session_id: &SessionId,
        new_role: ConnectionRole,
    ) -> Result<()> {
        // Validate target session exists
        if !self.sessions.contains_key(target_session_id) {
            return Err(Error::SessionNotFound(target_session_id.to_string()));
        }

        // Validate target session is not draining
        if self.is_draining(target_session_id) {
            return Err(Error::Connection(format!(
                "target session {target_session_id} is draining, cannot migrate"
            )));
        }

        // Unbind from source session
        let source_session_id = self.unbind_connection(conn_id).ok_or_else(|| {
            Error::Connection(format!("connection {conn_id} not bound to any session"))
        })?;

        // Bind to target session
        match self.bind_connection(target_session_id, conn_id.clone(), new_role) {
            Ok(()) => {
                info!(
                    conn_id = %conn_id,
                    from_session = %source_session_id,
                    to_session = %target_session_id,
                    new_role = ?new_role,
                    "connection migrated"
                );
                Ok(())
            }
            Err(e) => {
                // Rollback: try to rebind to source session as Streaming
                warn!(
                    conn_id = %conn_id,
                    target_session = %target_session_id,
                    error = %e,
                    "migration failed, attempting rollback"
                );
                let _ = self.bind_connection(
                    &source_session_id,
                    conn_id.clone(),
                    ConnectionRole::Streaming,
                );
                Err(e)
            }
        }
    }

    /// Record bytes received for a session.
    pub fn record_bytes_in(&self, session_id: &SessionId, bytes: u64) {
        self.session_metrics
            .entry(session_id.clone())
            .or_insert_with(SessionMetrics::new)
            .bytes_in
            .fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record bytes sent for a session.
    pub fn record_bytes_out(&self, session_id: &SessionId, bytes: u64) {
        self.session_metrics
            .entry(session_id.clone())
            .or_insert_with(SessionMetrics::new)
            .bytes_out
            .fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record a message for a session.
    pub fn record_message(&self, session_id: &SessionId) {
        self.session_metrics
            .entry(session_id.clone())
            .or_insert_with(SessionMetrics::new)
            .message_count
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Get metrics snapshot for a session.
    pub fn get_metrics(&self, session_id: &SessionId) -> Option<SessionMetricsSnapshot> {
        self.session_metrics.get(session_id).map(|m| m.snapshot())
    }

    /// Start draining a session (reject new connections).
    ///
    /// A draining session will not accept new connections via `bind_connection`,
    /// but existing connections remain active.
    pub fn drain_session(&self, session_id: &SessionId) -> Result<()> {
        if !self.sessions.contains_key(session_id) {
            return Err(Error::SessionNotFound(session_id.to_string()));
        }

        self.draining.insert(session_id.clone(), true);
        info!(session_id = %session_id, "session is now draining");
        Ok(())
    }

    /// Check if a session is draining.
    pub fn is_draining(&self, session_id: &SessionId) -> bool {
        self.draining
            .get(session_id)
            .map(|r| *r.value())
            .unwrap_or(false)
    }
}

// ────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_manager() -> SessionManager {
        let config = SessionConfig::default();
        let event_bus = Arc::new(EventBus::default());
        SessionManager::new(config, event_bus)
    }

    #[test]
    fn test_create_and_get_session() {
        let manager = make_manager();
        let client_id = ClientId::new();

        let session_id = manager.create_session(client_id.clone());
        assert_eq!(manager.session_count(), 1);

        let session = manager.get_session(&session_id).unwrap();
        assert_eq!(session.client_id, client_id);
        assert!(session.is_empty());
    }

    #[test]
    fn test_bind_unbind_connection() {
        let manager = make_manager();
        let client_id = ClientId::new();
        let session_id = manager.create_session(client_id);

        let conn_id = ConnectionId::new();
        manager
            .bind_connection(&session_id, conn_id.clone(), ConnectionRole::Primary)
            .unwrap();
        assert_eq!(manager.connection_count(), 1);

        {
            let session = manager.get_session(&session_id).unwrap();
            assert_eq!(session.connection_count(), 1);
            assert!(session.has_primary());
        }

        // Unbind
        let returned_sid = manager.unbind_connection(&conn_id);
        assert_eq!(returned_sid, Some(session_id.clone()));
        assert_eq!(manager.connection_count(), 0);

        let session = manager.get_session(&session_id).unwrap();
        assert!(session.is_empty());
    }

    #[test]
    fn test_get_session_by_connection() {
        let manager = make_manager();
        let session_id = manager.create_session(ClientId::new());

        let conn_id = ConnectionId::new();
        manager
            .bind_connection(&session_id, conn_id.clone(), ConnectionRole::Primary)
            .unwrap();

        let found = manager.get_session_by_connection(&conn_id);
        assert_eq!(found, Some(session_id));

        // Unknown connection
        let unknown = manager.get_session_by_connection(&ConnectionId::new());
        assert!(unknown.is_none());
    }

    #[test]
    fn test_destroy_session() {
        let manager = make_manager();
        let client_id = ClientId::new();
        let session_id = manager.create_session(client_id.clone());

        let conn_id = ConnectionId::new();
        manager
            .bind_connection(&session_id, conn_id.clone(), ConnectionRole::Primary)
            .unwrap();

        manager.destroy_session(&session_id, "test cleanup");

        assert_eq!(manager.session_count(), 0);
        assert_eq!(manager.connection_count(), 0);
        assert!(manager.get_session(&session_id).is_none());
        assert!(manager.get_session_by_connection(&conn_id).is_none());
        assert!(manager.get_session_by_client(&client_id).is_none());
    }

    #[test]
    fn test_session_count() {
        let manager = make_manager();

        assert_eq!(manager.session_count(), 0);
        let sid1 = manager.create_session(ClientId::new());
        assert_eq!(manager.session_count(), 1);
        let _sid2 = manager.create_session(ClientId::new());
        assert_eq!(manager.session_count(), 2);

        manager.destroy_session(&sid1, "done");
        assert_eq!(manager.session_count(), 1);
    }

    #[test]
    fn test_promote_connection() {
        let manager = make_manager();
        let session_id = manager.create_session(ClientId::new());

        let conn_id = ConnectionId::new();
        manager
            .bind_connection(&session_id, conn_id.clone(), ConnectionRole::Streaming)
            .unwrap();

        // Promote to Primary via manager
        manager
            .promote_connection(&conn_id, ConnectionRole::Primary)
            .unwrap();

        let session = manager.get_session(&session_id).unwrap();
        assert_eq!(session.get_role(&conn_id), Some(&ConnectionRole::Primary));
        assert!(session.has_primary());
    }

    #[test]
    fn test_demote_connection() {
        let manager = make_manager();
        let session_id = manager.create_session(ClientId::new());

        let conn_id = ConnectionId::new();
        manager
            .bind_connection(&session_id, conn_id.clone(), ConnectionRole::Primary)
            .unwrap();

        // Demote via manager
        manager.demote_connection(&conn_id).unwrap();

        let session = manager.get_session(&session_id).unwrap();
        assert_eq!(
            session.get_role(&conn_id),
            Some(&ConnectionRole::Streaming)
        );
        assert!(!session.has_primary());
    }

    #[test]
    fn test_migrate_connection() {
        let manager = make_manager();
        let session1 = manager.create_session(ClientId::new());
        let session2 = manager.create_session(ClientId::new());

        let conn_id = ConnectionId::new();
        manager
            .bind_connection(&session1, conn_id.clone(), ConnectionRole::Primary)
            .unwrap();

        // Migrate from session1 to session2
        manager
            .migrate_connection(&conn_id, &session2, ConnectionRole::Notification)
            .unwrap();

        // Connection should now be in session2
        assert_eq!(
            manager.get_session_by_connection(&conn_id),
            Some(session2.clone())
        );

        let s2 = manager.get_session(&session2).unwrap();
        assert!(s2.has_connection(&conn_id));
        assert_eq!(
            s2.get_role(&conn_id),
            Some(&ConnectionRole::Notification)
        );

        // session1 should no longer have it
        let s1 = manager.get_session(&session1).unwrap();
        assert!(!s1.has_connection(&conn_id));
    }

    #[test]
    fn test_session_metrics() {
        let manager = make_manager();
        let session_id = manager.create_session(ClientId::new());

        // Record some metrics
        manager.record_bytes_in(&session_id, 100);
        manager.record_bytes_in(&session_id, 50);
        manager.record_bytes_out(&session_id, 200);
        manager.record_message(&session_id);
        manager.record_message(&session_id);
        manager.record_message(&session_id);

        let snapshot = manager.get_metrics(&session_id).unwrap();
        assert_eq!(snapshot.bytes_in, 150);
        assert_eq!(snapshot.bytes_out, 200);
        assert_eq!(snapshot.message_count, 3);
        assert!(snapshot.connected_duration_secs >= 0);
    }

    #[test]
    fn test_drain_session() {
        let manager = make_manager();
        let session_id = manager.create_session(ClientId::new());

        // Not draining initially
        assert!(!manager.is_draining(&session_id));

        // Drain it
        manager.drain_session(&session_id).unwrap();
        assert!(manager.is_draining(&session_id));

        // New connections should be rejected
        let conn_id = ConnectionId::new();
        let result = manager.bind_connection(&session_id, conn_id, ConnectionRole::Streaming);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("draining"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_drain_and_existing_connections() {
        let manager = make_manager();
        let session_id = manager.create_session(ClientId::new());

        // Add a connection before draining
        let conn_id = ConnectionId::new();
        manager
            .bind_connection(&session_id, conn_id.clone(), ConnectionRole::Primary)
            .unwrap();

        // Drain the session
        manager.drain_session(&session_id).unwrap();

        // Existing connection should still be there
        let session = manager.get_session(&session_id).unwrap();
        assert!(session.has_connection(&conn_id));
        assert_eq!(session.connection_count(), 1);
        assert!(session.has_primary());
        drop(session);

        // But new connections should be rejected
        let new_conn = ConnectionId::new();
        let result =
            manager.bind_connection(&session_id, new_conn, ConnectionRole::Streaming);
        assert!(result.is_err());

        // Existing connection still works (can be unbound normally)
        let unbind_result = manager.unbind_connection(&conn_id);
        assert_eq!(unbind_result, Some(session_id));
    }
}
