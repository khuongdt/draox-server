use chrono::{DateTime, Utc};
use server_core::{ClientId, ConnectionId, ConnectionRole, Error, Result, SessionId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A server-authoritative client session that manages multiple connections.
///
/// Each client is assigned one session. A session can hold up to
/// `max_connections_per_session` connections, with at most one Primary
/// and one Control connection.
#[derive(Debug, Clone)]
pub struct ClientSession {
    pub session_id: SessionId,
    pub client_id: ClientId,
    pub connections: HashMap<ConnectionId, ConnectionRole>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub metadata: serde_json::Value,
    pub authenticated: bool,
}

impl ClientSession {
    /// Create a new session for the given client.
    pub fn new(client_id: ClientId) -> Self {
        let now = Utc::now();
        Self {
            session_id: SessionId::new(),
            client_id,
            connections: HashMap::new(),
            created_at: now,
            last_activity: now,
            metadata: serde_json::Value::Object(serde_json::Map::new()),
            authenticated: false,
        }
    }

    /// Add a connection with the given role to this session.
    ///
    /// Returns an error if:
    /// - The session already has a Primary connection and `role` is Primary
    /// - The session already has a Control connection and `role` is Control
    pub fn add_connection(&mut self, conn_id: ConnectionId, role: ConnectionRole) -> Result<()> {
        // Validate: max 1 Primary
        if role == ConnectionRole::Primary && self.has_primary() {
            return Err(Error::Connection(
                "session already has a primary connection".to_string(),
            ));
        }

        // Validate: max 1 Control
        if role == ConnectionRole::Control && self.has_control() {
            return Err(Error::Connection(
                "session already has a control connection".to_string(),
            ));
        }

        self.connections.insert(conn_id, role);
        self.last_activity = Utc::now();
        Ok(())
    }

    /// Remove a connection from this session. Returns true if it existed.
    pub fn remove_connection(&mut self, conn_id: &ConnectionId) -> bool {
        let removed = self.connections.remove(conn_id).is_some();
        if removed {
            self.last_activity = Utc::now();
        }
        removed
    }

    /// Check if this session contains the given connection.
    pub fn has_connection(&self, conn_id: &ConnectionId) -> bool {
        self.connections.contains_key(conn_id)
    }

    /// Number of connections in this session.
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// Whether this session has a Primary connection.
    pub fn has_primary(&self) -> bool {
        self.connections
            .values()
            .any(|r| *r == ConnectionRole::Primary)
    }

    /// Whether this session has a Control connection.
    fn has_control(&self) -> bool {
        self.connections
            .values()
            .any(|r| *r == ConnectionRole::Control)
    }

    /// Get the ConnectionId of the primary connection, if any.
    pub fn primary_connection(&self) -> Option<&ConnectionId> {
        self.connections
            .iter()
            .find(|(_, r)| **r == ConnectionRole::Primary)
            .map(|(id, _)| id)
    }

    /// Whether this session has no connections.
    pub fn is_empty(&self) -> bool {
        self.connections.is_empty()
    }

    /// Promote a connection to a new role.
    ///
    /// Rules: cannot promote to Primary if one already exists;
    /// cannot promote to Control if one already exists.
    pub fn promote_connection(
        &mut self,
        conn_id: &ConnectionId,
        new_role: ConnectionRole,
    ) -> Result<()> {
        // Verify the connection exists in this session
        if !self.connections.contains_key(conn_id) {
            return Err(Error::Connection(format!(
                "connection {conn_id} not found in session"
            )));
        }

        // Validate uniqueness constraints for the new role
        if new_role == ConnectionRole::Primary && self.has_primary() {
            // Check if it's the same connection already holding Primary
            if self.connections.get(conn_id) != Some(&ConnectionRole::Primary) {
                return Err(Error::Connection(
                    "session already has a primary connection".to_string(),
                ));
            }
        }

        if new_role == ConnectionRole::Control && self.has_control() {
            if self.connections.get(conn_id) != Some(&ConnectionRole::Control) {
                return Err(Error::Connection(
                    "session already has a control connection".to_string(),
                ));
            }
        }

        self.connections.insert(conn_id.clone(), new_role);
        self.last_activity = Utc::now();
        Ok(())
    }

    /// Demote a connection to Streaming role.
    pub fn demote_connection(&mut self, conn_id: &ConnectionId) -> Result<()> {
        if !self.connections.contains_key(conn_id) {
            return Err(Error::Connection(format!(
                "connection {conn_id} not found in session"
            )));
        }

        self.connections
            .insert(conn_id.clone(), ConnectionRole::Streaming);
        self.last_activity = Utc::now();
        Ok(())
    }

    /// Get the role of a specific connection.
    pub fn get_role(&self, conn_id: &ConnectionId) -> Option<&ConnectionRole> {
        self.connections.get(conn_id)
    }

    /// Update last_activity to now.
    pub fn touch(&mut self) {
        self.last_activity = Utc::now();
    }
}

/// Summary information about a session, used for listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: SessionId,
    pub client_id: ClientId,
    pub connection_count: usize,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub authenticated: bool,
}

impl From<&ClientSession> for SessionInfo {
    fn from(s: &ClientSession) -> Self {
        Self {
            session_id: s.session_id.clone(),
            client_id: s.client_id.clone(),
            connection_count: s.connection_count(),
            created_at: s.created_at,
            last_activity: s.last_activity,
            authenticated: s.authenticated,
        }
    }
}

// ────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_session() {
        let client_id = ClientId::new();
        let session = ClientSession::new(client_id.clone());

        assert_eq!(session.client_id, client_id);
        assert!(session.is_empty());
        assert_eq!(session.connection_count(), 0);
        assert!(!session.authenticated);
        assert!(!session.has_primary());
    }

    #[test]
    fn test_add_connection() {
        let mut session = ClientSession::new(ClientId::new());
        let conn_id = ConnectionId::new();

        session
            .add_connection(conn_id.clone(), ConnectionRole::Primary)
            .unwrap();

        assert!(session.has_connection(&conn_id));
        assert_eq!(session.connection_count(), 1);
        assert!(session.has_primary());
        assert_eq!(session.primary_connection(), Some(&conn_id));
    }

    #[test]
    fn test_role_validation_max_one_primary() {
        let mut session = ClientSession::new(ClientId::new());

        session
            .add_connection(ConnectionId::new(), ConnectionRole::Primary)
            .unwrap();

        // Second Primary should fail
        let result = session.add_connection(ConnectionId::new(), ConnectionRole::Primary);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string()
                .contains("session already has a primary connection"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_role_validation_max_one_control() {
        let mut session = ClientSession::new(ClientId::new());

        session
            .add_connection(ConnectionId::new(), ConnectionRole::Control)
            .unwrap();

        // Second Control should fail
        let result = session.add_connection(ConnectionId::new(), ConnectionRole::Control);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string()
                .contains("session already has a control connection"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_remove_connection() {
        let mut session = ClientSession::new(ClientId::new());
        let conn_id = ConnectionId::new();

        session
            .add_connection(conn_id.clone(), ConnectionRole::Primary)
            .unwrap();
        assert!(!session.is_empty());

        let removed = session.remove_connection(&conn_id);
        assert!(removed);
        assert!(session.is_empty());
        assert!(!session.has_primary());

        // Removing again should return false
        let removed_again = session.remove_connection(&conn_id);
        assert!(!removed_again);
    }

    #[test]
    fn test_is_empty() {
        let mut session = ClientSession::new(ClientId::new());
        assert!(session.is_empty());

        let conn_id = ConnectionId::new();
        session
            .add_connection(conn_id.clone(), ConnectionRole::Streaming)
            .unwrap();
        assert!(!session.is_empty());

        session.remove_connection(&conn_id);
        assert!(session.is_empty());
    }

    #[test]
    fn test_promote_connection() {
        let mut session = ClientSession::new(ClientId::new());
        let conn_id = ConnectionId::new();

        // Add as Streaming, then promote to Primary
        session
            .add_connection(conn_id.clone(), ConnectionRole::Streaming)
            .unwrap();
        assert_eq!(
            session.get_role(&conn_id),
            Some(&ConnectionRole::Streaming)
        );

        session
            .promote_connection(&conn_id, ConnectionRole::Primary)
            .unwrap();
        assert_eq!(session.get_role(&conn_id), Some(&ConnectionRole::Primary));
        assert!(session.has_primary());
    }

    #[test]
    fn test_promote_to_existing_primary_fails() {
        let mut session = ClientSession::new(ClientId::new());
        let conn1 = ConnectionId::new();
        let conn2 = ConnectionId::new();

        session
            .add_connection(conn1.clone(), ConnectionRole::Primary)
            .unwrap();
        session
            .add_connection(conn2.clone(), ConnectionRole::Streaming)
            .unwrap();

        // Promoting conn2 to Primary should fail because conn1 is already Primary
        let result = session.promote_connection(&conn2, ConnectionRole::Primary);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string()
                .contains("session already has a primary connection"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_demote_connection() {
        let mut session = ClientSession::new(ClientId::new());
        let conn_id = ConnectionId::new();

        session
            .add_connection(conn_id.clone(), ConnectionRole::Primary)
            .unwrap();
        assert!(session.has_primary());

        session.demote_connection(&conn_id).unwrap();
        assert_eq!(
            session.get_role(&conn_id),
            Some(&ConnectionRole::Streaming)
        );
        assert!(!session.has_primary());
    }

    #[test]
    fn test_get_role() {
        let mut session = ClientSession::new(ClientId::new());
        let conn1 = ConnectionId::new();
        let conn2 = ConnectionId::new();
        let unknown = ConnectionId::new();

        session
            .add_connection(conn1.clone(), ConnectionRole::Primary)
            .unwrap();
        session
            .add_connection(conn2.clone(), ConnectionRole::Notification)
            .unwrap();

        assert_eq!(session.get_role(&conn1), Some(&ConnectionRole::Primary));
        assert_eq!(
            session.get_role(&conn2),
            Some(&ConnectionRole::Notification)
        );
        assert_eq!(session.get_role(&unknown), None);
    }
}
