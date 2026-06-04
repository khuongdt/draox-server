use crate::registry::PluginRegistry;
use chrono::Utc;
use connection_manager::{manager::SessionManager, session_auth::AuthInfo};
use jsonwebtoken::{decode, DecodingKey, Validation};
use plugin_sdk::context::EventBusHandle;
use plugin_sdk::traits::WsActionContext;
use plugin_sdk::Identity;
use serde::Deserialize;
use server_core::event::{EventBus, ServerEvent};
use server_core::{ConnectionId, Error, Result};
use sha2::{Digest, Sha256};
use socket_server::WsActionDispatcher;
use socket_server::handler::BoxFuture;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, warn};

/// Minimal JWT claims for WebSocket auth — defined locally to avoid
/// visibility issues with connection-manager's pub(crate) type.
#[derive(Debug, Deserialize)]
struct WsJwtClaims {
    pub sub: String,
    pub role: String,
    pub exp: u64,
}

/// Concrete `WsActionDispatcher` (the trait lives in `socket-server`).
///
/// Wraps a `PluginRegistry`, `EventBus`, and optional `SessionManager`.
/// When a `SessionManager` is provided, the dispatcher:
///
/// - Handles `action == "auth"` by validating the JWT and authenticating the session.
/// - Enforces the auth gate: non-auth actions from unauthenticated sessions are rejected.
/// - Builds a real `Identity` from the authenticated session so plugins never see JWT tokens.
///
/// If no `SessionManager` is provided (e.g. in tests), behaviour falls back to
/// anonymous identity and no auth gate.
pub struct PluginWsDispatcher {
    registry: Arc<PluginRegistry>,
    event_bus: Arc<EventBus>,
    /// Session manager for auth lookup and token validation (optional).
    session_manager: Option<Arc<SessionManager>>,
    /// JWT secret for WebSocket auth (same secret used by admin-api login).
    jwt_secret: String,
    /// Whether to reject unauthenticated non-auth actions.
    require_auth: bool,
}

impl PluginWsDispatcher {
    /// Create a dispatcher without session-level authentication (legacy / tests).
    pub fn new(registry: Arc<PluginRegistry>, event_bus: Arc<EventBus>) -> Self {
        Self {
            registry,
            event_bus,
            session_manager: None,
            jwt_secret: String::new(),
            require_auth: false,
        }
    }

    /// Create a dispatcher with full auth support.
    ///
    /// `jwt_secret` must match the secret used to issue tokens via
    /// `POST /api/auth/login` (typically `config.admin_api.jwt_secret`).
    pub fn new_with_auth(
        registry: Arc<PluginRegistry>,
        event_bus: Arc<EventBus>,
        session_manager: Arc<SessionManager>,
        jwt_secret: String,
        require_auth: bool,
    ) -> Self {
        Self {
            registry,
            event_bus,
            session_manager: Some(session_manager),
            jwt_secret,
            require_auth,
        }
    }

    // ── Helpers ─────────────────────────────────────────────────────────────

    fn validate_jwt(&self, token: &str) -> std::result::Result<WsJwtClaims, String> {
        if self.jwt_secret.is_empty() {
            return Err("no JWT secret configured".into());
        }
        let key = DecodingKey::from_secret(self.jwt_secret.as_bytes());
        let mut validation = Validation::default();
        validation.validate_exp = true;
        decode::<WsJwtClaims>(token, &key, &validation)
            .map(|d| d.claims)
            .map_err(|e: jsonwebtoken::errors::Error| e.to_string())
    }

    fn sha256_hex(token: &str) -> String {
        format!("{:x}", Sha256::digest(token.as_bytes()))
    }

    /// Build an `Identity` from the session's auth state (or return anonymous).
    fn build_identity(&self, conn_id: &ConnectionId) -> Identity {
        if let Some(mgr) = &self.session_manager {
            if let Some(session_id) = mgr.get_session_by_connection(conn_id) {
                if let Some(auth) = mgr.get_session_auth(&session_id) {
                    return Identity::new(
                        auth.user_id,
                        auth.roles.first().cloned().unwrap_or_else(|| "viewer".into()),
                    );
                }
            }
        }
        Identity::anonymous()
    }

    /// Handle `action == "auth"` for WebSocket connections.
    async fn handle_auth_action(
        &self,
        payload: serde_json::Value,
        conn_id: &ConnectionId,
    ) -> Result<serde_json::Value> {
        let mgr = self
            .session_manager
            .as_ref()
            .ok_or_else(|| Error::Config("session manager not configured".into()))?;

        let token = payload
            .get("token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::AuthFailed("missing 'token' in auth payload".into()))?;

        let claims = self
            .validate_jwt(token)
            .map_err(|e| Error::AuthFailed(e))?;

        let session_id = mgr
            .get_session_by_connection(conn_id)
            .ok_or_else(|| Error::SessionNotFound("session not found for connection".into()))?;

        let token_hash = Self::sha256_hex(token);
        mgr.authenticate_session(
            &session_id,
            AuthInfo {
                user_id: claims.sub.clone(),
                roles: vec![claims.role.clone()],
                authenticated_at: Utc::now(),
                token_hash,
            },
        );

        info!(
            conn_id = %conn_id,
            session_id = %session_id,
            user_id = %claims.sub,
            role = %claims.role,
            "WebSocket connection authenticated"
        );

        Ok(serde_json::json!({ "session_id": session_id.to_string() }))
    }
}

impl WsActionDispatcher for PluginWsDispatcher {
    fn dispatch<'a>(
        &'a self,
        action: String,
        payload: serde_json::Value,
        connection_id: &'a ConnectionId,
    ) -> BoxFuture<'a, Result<serde_json::Value>> {
        let conn_id = connection_id.clone();
        Box::pin(async move {
            // ── Handle built-in "auth" action ────────────────────────────────
            if action == "auth" {
                return self.handle_auth_action(payload, &conn_id).await;
            }

            // ── Auth gate: reject anonymous connections (if require_auth) ────
            let identity = self.build_identity(&conn_id);
            if identity.is_anonymous() && self.require_auth {
                warn!(
                    conn_id = %conn_id,
                    action = %action,
                    "WS action rejected — connection not authenticated"
                );
                return Err(Error::Unauthorized(
                    "role 'anonymous' only allows 'auth' and 'ping'".into(),
                ));
            }

            // ── Dispatch to plugin ───────────────────────────────────────────
            let events: Arc<dyn EventBusHandle> = Arc::new(EventBusBridge {
                bus: Arc::clone(&self.event_bus),
            });
            let ctx = WsActionContext {
                identity,
                connection_id: conn_id,
                events,
            };
            self.registry.dispatch_ws_action(&action, payload, ctx).await
        })
    }
}

/// Thin adapter so plugin code can call `ctx.events.publish(...)` /
/// `ctx.events.subscribe(...)` through the EventBusHandle trait without
/// caring that the concrete bus is `server_core::event::EventBus`.
struct EventBusBridge {
    bus: Arc<EventBus>,
}

impl EventBusHandle for EventBusBridge {
    fn publish(&self, event: ServerEvent) {
        self.bus.publish(event);
    }
    fn subscribe(&self, topic: &str) -> broadcast::Receiver<Arc<ServerEvent>> {
        self.bus.subscribe_topic(topic.to_string())
    }
}
