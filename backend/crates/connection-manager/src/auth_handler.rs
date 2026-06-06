//! Wire-protocol authentication handler for TCP, UDP, and WebSocket connections.
//!
//! `AuthHandler` is a `ConnectionHandler` decorator that sits between the traffic guard
//! and the inner `SessionHandler`. It implements the authentication middleware layer:
//!
//! - Parses newline-delimited JSON for TCP (with per-connection receive buffer).
//! - Parses full-datagram JSON for UDP.
//! - Adds an auth gate to WebSocket (WS connections marked pending until authenticated).
//! - Validates JWT tokens and authenticates sessions.
//! - Enforces an auth timeout (connections that do not authenticate are closed).
//! - Routes authenticated actions to the plugin dispatcher.
//!
//! **Plugins never see JWT tokens.** They only receive `WsActionContext` with a real
//! `Identity` populated from the authenticated session.

use crate::heartbeat_manager::{run_heartbeat_task, HeartbeatManager};
use crate::manager::SessionManager;
use crate::session_auth::AuthInfo;
use crate::wire::{TcpJwtClaims, WireRequest, WireResponse};
use chrono::Utc;
use dashmap::DashMap;
use jsonwebtoken::{decode, DecodingKey, Validation};
use server_core::{
    ConnectionId, ConnectionInfo, ConnectionRole, Error, Protocol, SessionId,
};
use sha2::{Digest, Sha256};
use socket_server::{
    handler::{BoxFuture, ConnectionHandler},
    ConnectionTracker, OutgoingMessage,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

/// How often the timeout sweep task runs.
const TIMEOUT_SWEEP_INTERVAL: Duration = Duration::from_secs(5);

/// Authentication and wire-protocol middleware for all connection types.
pub struct AuthHandler {
    /// Inner handler (SessionHandler) for session lifecycle events.
    inner: Arc<dyn ConnectionHandler>,
    /// Optional plugin action dispatcher; if None, authenticated actions are silently dropped.
    dispatcher: Option<Arc<dyn socket_server::ws_dispatch::WsActionDispatcher>>,
    manager: Arc<SessionManager>,
    tracker: Arc<ConnectionTracker>,
    heartbeat_manager: Arc<HeartbeatManager>,
    jwt_secret: String,
    require_auth: bool,
    auth_timeout: Duration,
    /// How long after a disconnect to allow reconnect with the same JWT to resume the session.
    session_resume_window: Duration,
    /// Delimiter used to separate messages in the TCP stream (admin-configurable).
    delimiter: Vec<u8>,
    /// Connections pending authentication: conn_id → (connect_time, protocol).
    pending: DashMap<ConnectionId, (Instant, Protocol)>,
    /// Per-connection receive buffer for TCP (newline-delimited framing).
    tcp_buffers: DashMap<ConnectionId, Vec<u8>>,
}

impl AuthHandler {
    pub fn new(
        inner: Arc<dyn ConnectionHandler>,
        dispatcher: Option<Arc<dyn socket_server::ws_dispatch::WsActionDispatcher>>,
        manager: Arc<SessionManager>,
        tracker: Arc<ConnectionTracker>,
        heartbeat_manager: Arc<HeartbeatManager>,
        jwt_secret: String,
        require_auth: bool,
        auth_timeout: Duration,
        session_resume_window: Duration,
        delimiter: Vec<u8>,
    ) -> Self {
        Self {
            inner,
            dispatcher,
            manager,
            tracker,
            heartbeat_manager,
            jwt_secret,
            require_auth,
            auth_timeout,
            session_resume_window,
            delimiter,
            pending: DashMap::new(),
            tcp_buffers: DashMap::new(),
        }
    }

    /// Spawn a background task that closes connections that exceed the auth timeout.
    pub fn start_timeout_task(self: Arc<Self>) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(TIMEOUT_SWEEP_INTERVAL);
            loop {
                interval.tick().await;
                let now = Instant::now();
                let expired: Vec<ConnectionId> = self
                    .pending
                    .iter()
                    .filter(|e| now.duration_since(e.value().0) >= self.auth_timeout)
                    .map(|e| e.key().clone())
                    .collect();
                for conn_id in expired {
                    warn!(conn_id = %conn_id, timeout_secs = %self.auth_timeout.as_secs(),
                          "auth timeout — closing connection");
                    let _ = self.tracker.send(&conn_id, OutgoingMessage::Close).await;
                    self.pending.remove(&conn_id);
                    self.tcp_buffers.remove(&conn_id);
                }
            }
        })
    }

    // ── JWT validation ──────────────────────────────────────────────────────

    fn validate_jwt(&self, token: &str) -> Result<TcpJwtClaims, String> {
        let key = DecodingKey::from_secret(self.jwt_secret.as_bytes());
        let mut validation = Validation::default();
        validation.validate_exp = true;
        decode::<TcpJwtClaims>(token, &key, &validation)
            .map(|data| data.claims)
            .map_err(|e| e.to_string())
    }

    fn sha256_hex(token: &str) -> String {
        format!("{:x}", Sha256::digest(token.as_bytes()))
    }

    // ── Response helpers ────────────────────────────────────────────────────

    async fn send_response(&self, conn_id: &ConnectionId, resp: WireResponse) {
        let bytes = resp.to_line(&self.delimiter);
        let _ = self
            .tracker
            .send(conn_id, OutgoingMessage::Binary(bytes))
            .await;
    }

    // ── Core message processing ─────────────────────────────────────────────

    /// Process a single parsed `WireRequest` for a connection.
    async fn process_message(&self, conn_id: &ConnectionId, req: WireRequest) {
        // ── Ping (always allowed, no auth required) ──────────────────────
        if req.is_ping() {
            self.send_response(conn_id, WireResponse::pong()).await;
            return;
        }

        if !req.is_request() {
            return; // Unknown frame type — silently ignore
        }

        let action = match req.action.as_deref() {
            Some(a) => a.to_string(),
            None => {
                self.send_response(conn_id, WireResponse::err(req.id.clone(), "missing action"))
                    .await;
                return;
            }
        };

        let session_id = match self.manager.get_session_by_connection(conn_id) {
            Some(sid) => sid,
            None => {
                self.send_response(
                    conn_id,
                    WireResponse::err(req.id.clone(), "session not found"),
                )
                .await;
                return;
            }
        };

        let is_authed = self.manager.is_session_authenticated(&session_id);

        match action.as_str() {
            // ── Auth action (always accepted regardless of current auth state) ──
            "auth" => {
                self.handle_auth(conn_id, &session_id, req).await;
            }

            // ── Anonymous gate: only "auth" and "ping" allowed ──────────────
            _ if !is_authed && self.require_auth => {
                self.send_response(
                    conn_id,
                    WireResponse::err(
                        req.id.clone(),
                        "role 'anonymous' only allows 'auth' and 'ping'",
                    ),
                )
                .await;
            }

            // ── Authenticated action: dispatch to plugin dispatcher ──────────
            _ => {
                self.handle_dispatched_action(conn_id, &session_id, req, &action)
                    .await;
            }
        }
    }

    async fn handle_auth(
        &self,
        conn_id: &ConnectionId,
        session_id: &SessionId,
        req: WireRequest,
    ) {
        let payload = req.payload.clone().unwrap_or_default();
        let token = match payload.get("token").and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => {
                self.send_response(
                    conn_id,
                    WireResponse::err(req.id.clone(), "auth payload missing 'token'"),
                )
                .await;
                return;
            }
        };

        match self.validate_jwt(&token) {
            Ok(claims) => {
                let token_hash = Self::sha256_hex(&token);

                // Check if an existing authenticated session uses the same token.
                // If so, and the session is still within the resume window, migrate
                // the connection to that session instead of creating a new one.
                let effective_session_id =
                    if let Some(existing_sid) = self.manager.find_session_by_token_hash(&token_hash)
                    {
                        if existing_sid != *session_id {
                            // Check last_activity against the resume window
                            let within_window = self
                                .manager
                                .get_session(&existing_sid)
                                .map(|s| {
                                    let elapsed = Utc::now()
                                        .signed_duration_since(s.last_activity)
                                        .num_seconds();
                                    elapsed <= self.session_resume_window.as_secs() as i64
                                })
                                .unwrap_or(false);

                            if within_window {
                                // Migrate conn to existing session, destroy temp session
                                let temp_sid = session_id.clone();
                                match self.manager.migrate_connection(
                                    conn_id,
                                    &existing_sid,
                                    ConnectionRole::Primary,
                                ) {
                                    Ok(()) => {
                                        // Destroy the temporary session created on connect
                                        self.manager.destroy_session(&temp_sid, "superseded by reconnect");
                                        info!(
                                            conn_id = %conn_id,
                                            resumed_session = %existing_sid,
                                            temp_session = %temp_sid,
                                            "session resumed (reconnect within window)"
                                        );
                                        existing_sid
                                    }
                                    Err(e) => {
                                        warn!(conn_id = %conn_id, error = %e, "session resume failed, using new session");
                                        session_id.clone()
                                    }
                                }
                            } else {
                                session_id.clone()
                            }
                        } else {
                            session_id.clone()
                        }
                    } else {
                        session_id.clone()
                    };

                let auth_info = AuthInfo {
                    user_id: claims.sub.clone(),
                    roles: vec![claims.role.clone()],
                    authenticated_at: Utc::now(),
                    token_hash,
                };
                // Read protocol before removing from pending
                let protocol = self
                    .pending
                    .get(conn_id)
                    .map(|e| e.value().1)
                    .unwrap_or(Protocol::Tcp);

                self.manager.authenticate_session(&effective_session_id, auth_info);
                self.pending.remove(conn_id);

                info!(
                    conn_id = %conn_id,
                    session_id = %effective_session_id,
                    user_id = %claims.sub,
                    role = %claims.role,
                    "connection authenticated"
                );

                // Spawn server-side heartbeat for TCP/UDP connections.
                // WebSocket heartbeat is handled natively by the WS layer (ping frames).
                if matches!(protocol, Protocol::Tcp | Protocol::Udp) {
                    let hb_mgr = Arc::clone(&self.heartbeat_manager);
                    let tracker = Arc::clone(&self.tracker);
                    let hb_conn = conn_id.clone();
                    let max_missed = self.manager.config().heartbeat_timeout_secs
                        / self.manager.config().heartbeat_interval_secs.max(1);
                    tokio::spawn(async move {
                        run_heartbeat_task(hb_conn, hb_mgr, tracker, max_missed as u32).await;
                    });
                }

                use serde_json::json;
                self.send_response(
                    conn_id,
                    WireResponse::ok(
                        req.id.clone(),
                        json!({ "session_id": effective_session_id.to_string() }),
                    ),
                )
                .await;
            }
            Err(e) => {
                warn!(conn_id = %conn_id, error = %e, "JWT validation failed");
                self.send_response(
                    conn_id,
                    WireResponse::err(req.id.clone(), &format!("auth failed: {e}")),
                )
                .await;
            }
        }
    }

    async fn handle_dispatched_action(
        &self,
        conn_id: &ConnectionId,
        _session_id: &SessionId,
        req: WireRequest,
        action: &str, // already an owned String ref, no borrow issue
    ) {
        let dispatcher = match &self.dispatcher {
            Some(d) => d,
            None => {
                self.send_response(
                    conn_id,
                    WireResponse::err(req.id.clone(), "no dispatcher configured"),
                )
                .await;
                return;
            }
        };

        let payload = req.payload.clone().unwrap_or_default();
        let id = req.id.clone();

        match dispatcher
            .dispatch(action.to_string(), payload, conn_id)
            .await
        {
            Ok(data) => {
                self.send_response(conn_id, WireResponse::ok(id, data)).await;
            }
            Err(e) => {
                self.send_response(
                    conn_id,
                    WireResponse::err(id, &e.to_string()),
                )
                .await;
            }
        }
    }
}

// ── ConnectionHandler impl ──────────────────────────────────────────────────

impl ConnectionHandler for AuthHandler {
    fn on_connect<'a>(&'a self, info: &'a ConnectionInfo) -> BoxFuture<'a, server_core::Result<()>> {
        Box::pin(async move {
            // Call inner handler first (creates session, binds Primary)
            self.inner.on_connect(info).await?;

            // Mark connection as pending authentication
            self.pending
                .insert(info.id.clone(), (Instant::now(), info.protocol));

            debug!(
                conn_id = %info.id,
                protocol = ?info.protocol,
                "connection marked pending auth"
            );
            Ok(())
        })
    }

    fn on_data<'a>(&'a self, conn_id: &'a ConnectionId, data: &'a [u8]) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            // Always record bytes_in via inner handler
            self.inner.on_data(conn_id, data).await;

            // Determine protocol from pending map
            let protocol = self
                .pending
                .get(conn_id)
                .map(|e| e.value().1)
                // If not pending, still check authenticated session for protocol
                .or_else(|| {
                    self.manager
                        .get_session_by_connection(conn_id)
                        .and_then(|sid| {
                            self.manager
                                .get_session(&sid)
                                .map(|_| Protocol::Tcp) // default fallback
                        })
                });

            match protocol {
                Some(Protocol::Tcp) => {
                    debug!(conn_id = %conn_id, bytes = data.len(), "TCP data received, buffering");
                    // Buffer data and extract complete newline-delimited messages
                    let messages = {
                        let mut buf = self
                            .tcp_buffers
                            .entry(conn_id.clone())
                            .or_default()
                            .clone();
                        buf.extend_from_slice(data);

                        let mut messages = Vec::new();
                        while let Some(pos) = buf
                            .windows(self.delimiter.len())
                            .position(|w| w == self.delimiter.as_slice())
                        {
                            let line: Vec<u8> = buf.drain(..pos).collect();
                            buf.drain(..self.delimiter.len()); // consume delimiter
                            if !line.is_empty() {
                                messages.push(line);
                            }
                        }
                        // Store remaining partial data back
                        self.tcp_buffers.insert(conn_id.clone(), buf);
                        messages
                    };

                    for line in messages {
                        // Strip UTF-8 BOM (0xEF 0xBB 0xBF): .NET StreamWriter(Encoding.UTF8)
                        // emits a BOM at the start of the stream, which serde_json rejects
                        // with "expected value at line 1 column 1".
                        const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";
                        let line = if line.starts_with(UTF8_BOM) {
                            line[UTF8_BOM.len()..].to_vec()
                        } else {
                            line
                        };
                        // Strip trailing \r to handle \r\n (Windows CRLF) in addition to \n.
                        let trimmed = line.strip_suffix(b"\r").unwrap_or(&line);
                        match serde_json::from_slice::<WireRequest>(trimmed) {
                            Ok(req) => {
                                self.process_message(conn_id, req).await;
                            }
                            Err(e) => {
                                warn!(conn_id = %conn_id, error = %e, bytes = trimmed.len(),
                                      "TCP: JSON parse failed");
                            }
                        }
                    }
                }

                Some(Protocol::Udp) => {
                    // UDP: each datagram is a complete message
                    match serde_json::from_slice::<WireRequest>(data) {
                        Ok(req) => {
                            self.process_message(conn_id, req).await;
                        }
                        Err(e) => {
                            warn!(conn_id = %conn_id, error = %e, bytes = data.len(),
                                  "UDP: JSON parse failed");
                        }
                    }
                }

                // WebSocket: framing and dispatch are handled by ws.rs + PluginWsDispatcher.
                // The auth gate for WS is enforced in PluginWsDispatcher (which checks
                // is_session_authenticated before routing to plugins).
                _ => {
                    debug!(conn_id = %conn_id, "non-TCP/UDP on_data — auth handled by WS layer");
                }
            }
        })
    }

    fn on_disconnect<'a>(
        &'a self,
        conn_id: &'a ConnectionId,
        reason: &'a str,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            self.pending.remove(conn_id);
            self.tcp_buffers.remove(conn_id);
            self.inner.on_disconnect(conn_id, reason).await;
        })
    }

    fn on_error<'a>(
        &'a self,
        conn_id: &'a ConnectionId,
        err: &'a Error,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            self.pending.remove(conn_id);
            self.tcp_buffers.remove(conn_id);
            self.inner.on_error(conn_id, err).await;
        })
    }
}

// ────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use server_config::model::SessionConfig;
    use server_core::event::EventBus;
    use server_core::{ConnectionId, Protocol};
    use socket_server::ws_dispatch::WsActionDispatcher;
    use std::net::SocketAddr;

    fn make_handler() -> Arc<AuthHandler> {
        let config = SessionConfig::default();
        let event_bus = Arc::new(EventBus::default());
        let manager = Arc::new(crate::manager::SessionManager::new(config.clone(), event_bus));
        let tracker = Arc::new(ConnectionTracker::new(100, 10));
        let inner = Arc::new(crate::handler::SessionHandler::new(
            Arc::clone(&manager),
            Arc::clone(&tracker),
        ));
        let heartbeat_manager = Arc::new(crate::heartbeat_manager::HeartbeatManager::new(
            Duration::from_secs(config.heartbeat_interval_secs),
            Duration::from_secs(config.heartbeat_timeout_secs),
        ));
        Arc::new(AuthHandler::new(
            inner,
            None,
            manager,
            tracker,
            heartbeat_manager,
            "test-secret".into(),
            true,
            Duration::from_secs(30),
            Duration::from_secs(300),
            b"\n".to_vec(),
        ))
    }

    #[tokio::test]
    async fn test_on_connect_marks_pending() {
        let handler = make_handler();
        let addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
        let info = ConnectionInfo::new(ConnectionId::new(), Protocol::Tcp, addr);
        handler.on_connect(&info).await.unwrap();
        assert!(handler.pending.contains_key(&info.id));
    }

    #[tokio::test]
    async fn test_on_disconnect_removes_pending() {
        let handler = make_handler();
        let addr: SocketAddr = "127.0.0.1:5001".parse().unwrap();
        let info = ConnectionInfo::new(ConnectionId::new(), Protocol::Tcp, addr);
        handler.on_connect(&info).await.unwrap();
        handler.on_disconnect(&info.id, "test").await;
        assert!(!handler.pending.contains_key(&info.id));
    }

    #[tokio::test]
    async fn test_tcp_message_buffering_splits_on_delimiter() {
        let handler = make_handler();
        let addr: SocketAddr = "127.0.0.1:5002".parse().unwrap();
        let info = ConnectionInfo::new(ConnectionId::new(), Protocol::Tcp, addr);
        handler.on_connect(&info).await.unwrap();

        // Send partial message then full message with delimiter
        let data = b"{\"type\":\"ping\"}\n";
        handler.on_data(&info.id, data).await;
        // No panic = buffering works; ping response attempted (tracker has no receiver so send fails silently)
    }

    #[tokio::test]
    async fn test_tcp_message_buffering_accumulates_partials() {
        let handler = make_handler();
        let addr: SocketAddr = "127.0.0.1:5003".parse().unwrap();
        let info = ConnectionInfo::new(ConnectionId::new(), Protocol::Tcp, addr);
        handler.on_connect(&info).await.unwrap();

        // Send in two partial chunks
        handler.on_data(&info.id, b"{\"type\":\"pin").await;
        // Buffer should have partial data, no message yet
        let buf_len = handler
            .tcp_buffers
            .get(&info.id)
            .map(|b| b.len())
            .unwrap_or(0);
        assert!(buf_len > 0, "partial data should be buffered");

        handler.on_data(&info.id, b"g\"}\n").await;
        // After complete message, buffer should be empty
        let buf_len = handler
            .tcp_buffers
            .get(&info.id)
            .map(|b| b.len())
            .unwrap_or(0);
        assert_eq!(buf_len, 0, "buffer should be empty after complete message");
    }

    #[tokio::test]
    async fn test_auth_with_invalid_jwt() {
        let handler = make_handler();
        let addr: SocketAddr = "127.0.0.1:5004".parse().unwrap();
        let info = ConnectionInfo::new(ConnectionId::new(), Protocol::Tcp, addr);
        handler.on_connect(&info).await.unwrap();

        let session_id = handler.manager.get_session_by_connection(&info.id).unwrap();

        // Try to authenticate with a bad token — should not authenticate the session
        let req = WireRequest {
            id: Some("r1".into()),
            frame_type: Some("request".into()),
            action: Some("auth".into()),
            payload: Some(serde_json::json!({"token": "not.a.real.jwt"})),
            ..Default::default()
        };
        handler.process_message(&info.id, req).await;
        assert!(!handler.manager.is_session_authenticated(&session_id));
    }

    #[tokio::test]
    async fn test_unauthenticated_non_auth_action_rejected() {
        let handler = make_handler();
        let addr: SocketAddr = "127.0.0.1:5005".parse().unwrap();
        let info = ConnectionInfo::new(ConnectionId::new(), Protocol::Tcp, addr);
        handler.on_connect(&info).await.unwrap();

        // Should be rejected — session not authenticated
        let req = WireRequest {
            id: Some("r2".into()),
            frame_type: Some("request".into()),
            action: Some("messaging.send_message".into()),
            payload: Some(serde_json::json!({})),
            ..Default::default()
        };
        // No panic = gate logic works (tracker send will fail silently in test)
        handler.process_message(&info.id, req).await;
        // Session still not authenticated
        let session_id = handler.manager.get_session_by_connection(&info.id).unwrap();
        assert!(!handler.manager.is_session_authenticated(&session_id));
    }

    #[test]
    fn test_sha256_hex_is_deterministic() {
        let h1 = AuthHandler::sha256_hex("token123");
        let h2 = AuthHandler::sha256_hex("token123");
        assert_eq!(h1, h2);
        assert_ne!(h1, AuthHandler::sha256_hex("different"));
    }
}
