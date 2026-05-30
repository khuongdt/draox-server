use crate::channel::{Channel, ChannelType};
use crate::message::{Message, MessageReaction, MessageType};
use crate::store::MessageStore;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use plugin_sdk::context::EventBusHandle;
use plugin_sdk::Identity;
use serde::{Deserialize, Serialize};
use serde_json::json;
use server_core::event::ServerEvent;
use server_core::ClientId;
use std::sync::Arc;

// ─────────────────────────────────────────────────────────────────────────────
// Route metadata (for OpenAPI/Swagger docs and capability discovery).
// Kept alongside the real handlers — the same source of truth for both.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessagingRouteInfo {
    pub method: &'static str,
    pub path: &'static str,
    pub description: &'static str,
}

pub fn messaging_routes() -> Vec<MessagingRouteInfo> {
    vec![
        MessagingRouteInfo { method: "GET",    path: "/api/channels",                  description: "List all available channels" },
        MessagingRouteInfo { method: "POST",   path: "/api/channels",                  description: "Create a new messaging channel" },
        MessagingRouteInfo { method: "GET",    path: "/api/channels/{id}",             description: "Get channel details by ID" },
        MessagingRouteInfo { method: "DELETE", path: "/api/channels/{id}",             description: "Delete a channel" },
        MessagingRouteInfo { method: "GET",    path: "/api/channels/{id}/messages",    description: "Get messages in a channel (paginated)" },
        MessagingRouteInfo { method: "POST",   path: "/api/channels/{id}/subscribe",   description: "Subscribe the current user to a channel" },
        MessagingRouteInfo { method: "POST",   path: "/api/channels/{id}/unsubscribe", description: "Unsubscribe the current user from a channel" },
        MessagingRouteInfo { method: "POST",   path: "/api/channels/{id}/typing",      description: "Send a typing indicator to a channel" },
        MessagingRouteInfo { method: "POST",   path: "/api/messages/send",             description: "Send a channel message" },
        MessagingRouteInfo { method: "GET",    path: "/api/messages/{id}",             description: "Get a message by its ID" },
        MessagingRouteInfo { method: "DELETE", path: "/api/messages/{id}",             description: "Delete a message by its ID" },
        MessagingRouteInfo { method: "PATCH",  path: "/api/messages/{id}",             description: "Edit the content of a message" },
        MessagingRouteInfo { method: "POST",   path: "/api/messages/{id}/react",       description: "Add a reaction emoji to a message" },
    ]
}

// ─────────────────────────────────────────────────────────────────────────────
// Router state — embedded into the plugin's Router via `.with_state(...)`.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct ApiState {
    store:  Arc<MessageStore>,
    events: Arc<dyn EventBusHandle>,
}

fn emit(events: &Arc<dyn EventBusHandle>, name: &str, payload: serde_json::Value) {
    events.publish(ServerEvent::Custom {
        source:  "messaging".to_string(),
        name:    name.to_string(),
        payload,
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// DTOs — match the TypeScript types in `tools/sdk-web/src/types.ts`.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ChannelDto {
    pub id:           String,
    pub name:         String,
    pub description:  String,
    pub created_by:   String,
    pub created_at:   String,
    pub channel_type: &'static str,
    pub topic:        String,
    pub is_system:    bool,
    pub frozen:       bool,
    pub member_count: usize,
}

impl From<Channel> for ChannelDto {
    fn from(c: Channel) -> Self {
        let channel_type = match c.channel_type {
            ChannelType::Public       => "Public",
            ChannelType::Private      => "Private",
            ChannelType::Direct       => "Direct",
            ChannelType::Announcement => "Announcement",
        };
        Self {
            member_count: c.subscribers.len(),
            id:           c.id,
            name:         c.name,
            description:  c.description,
            created_by:   c.created_by.as_str().to_string(),
            created_at:   c.created_at.to_rfc3339(),
            channel_type,
            topic:        c.topic,
            is_system:    c.is_system,
            frozen:       c.frozen,
        }
    }
}

#[derive(Serialize)]
pub struct ReactionDto {
    pub emoji: String,
    pub users: Vec<String>,
}

impl From<MessageReaction> for ReactionDto {
    fn from(r: MessageReaction) -> Self {
        Self { emoji: r.emoji, users: r.users }
    }
}

#[derive(Serialize)]
pub struct MessageDto {
    pub id:          String,
    pub channel_id:  String,
    pub sender_id:   String,
    pub text:        String,
    pub reply_to_id: Option<String>,
    pub sent_at:     String,
    pub edited_at:   Option<String>,
    pub reactions:   Vec<ReactionDto>,
}

impl From<Message> for MessageDto {
    fn from(m: Message) -> Self {
        Self {
            id:          m.id,
            channel_id:  m.to,
            sender_id:   m.from.as_str().to_string(),
            text:        m.content,
            reply_to_id: m.reply_to,
            sent_at:     m.timestamp.to_rfc3339(),
            edited_at:   m.edited_at.map(|t| t.to_rfc3339()),
            reactions:   m.reactions.into_iter().map(ReactionDto::from).collect(),
        }
    }
}

#[derive(Serialize)]
pub struct MessageHistoryDto {
    pub messages:  Vec<MessageDto>,
    pub has_more:  bool,
    pub oldest_id: Option<String>,
}

#[derive(Serialize)]
pub struct SendMessageResponseDto {
    pub message: MessageDto,
}

// ─────────────────────────────────────────────────────────────────────────────
// Response wrapper — matches admin-api::ApiResponse shape so the SDK's
// `fetchApi` correctly unwraps `{ success, data }`.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ApiResponse<T: Serialize> {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data:    Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    fn ok(data: T) -> Self {
        Self { success: true, data: Some(data), message: None }
    }
}

impl ApiResponse<()> {
    fn message(msg: impl Into<String>) -> Self {
        Self { success: true, data: None, message: Some(msg.into()) }
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

fn err_resp(status: StatusCode, msg: impl Into<String>) -> axum::response::Response {
    let body = serde_json::json!({
        "success": false,
        "error":   status.canonical_reason().unwrap_or("Unknown"),
        "message": msg.into(),
    });
    (status, Json(body)).into_response()
}

// ─────────────────────────────────────────────────────────────────────────────
// Request bodies / query strings.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateChannelRequest {
    pub name:        String,
    #[serde(default)]
    pub description: String,
}

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub limit:  Option<usize>,
    pub before: Option<String>,
}

#[derive(Deserialize)]
pub struct SendMessageRequest {
    pub channel_id:  String,
    pub text:        String,
    pub reply_to_id: Option<String>,
}

#[derive(Deserialize)]
pub struct EditMessageRequest {
    pub text: String,
}

#[derive(Deserialize)]
pub struct ReactionRequest {
    pub emoji: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Router constructor — what the plugin returns from `Plugin::http_router`.
// ─────────────────────────────────────────────────────────────────────────────

pub fn router(store: Arc<MessageStore>, events: Arc<dyn EventBusHandle>) -> Router {
    let state = ApiState { store, events };
    Router::new()
        // ── Channels ────────────────────────────────────────────────
        .route("/api/channels",                          get(list_channels).post(create_channel))
        .route("/api/channels/{id}",                     get(get_channel).delete(delete_channel))
        .route("/api/channels/{id}/messages",            get(get_channel_messages))
        .route("/api/channels/{id}/subscribe",           post(subscribe_channel))
        .route("/api/channels/{id}/unsubscribe",         post(unsubscribe_channel))
        .route("/api/channels/{id}/typing",              post(send_typing))
        .route("/api/channels/{id}/freeze",              post(freeze_channel))
        .route("/api/channels/{id}/unfreeze",            post(unfreeze_channel))
        // ── Messages ────────────────────────────────────────────────
        .route("/api/messages/send",                     post(send_message))
        .route("/api/messages/{id}",                     get(get_message).delete(delete_message).patch(edit_message))
        .route("/api/messages/{id}/react",               post(add_reaction))
        .with_state(state)
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

async fn list_channels(State(state): State<ApiState>) -> impl IntoResponse {
    let channels: Vec<ChannelDto> = state
        .store
        .list_channels()
        .into_iter()
        .map(ChannelDto::from)
        .collect();
    ApiResponse::ok(channels)
}

async fn create_channel(
    State(state): State<ApiState>,
    Extension(identity): Extension<Identity>,
    Json(req): Json<CreateChannelRequest>,
) -> axum::response::Response {
    if req.name.trim().is_empty() {
        return err_resp(StatusCode::BAD_REQUEST, "channel name must not be empty");
    }
    let creator = ClientId::from_str(&identity.user_id);
    let ch_id = state.store.create_channel(req.name, creator);
    match state.store.get_channel(&ch_id) {
        Some(ch) => {
            let dto = ChannelDto::from(ch);
            emit(&state.events, "channel_created", json!({ "channel": &dto }));
            ApiResponse::ok(dto).into_response()
        }
        None => err_resp(StatusCode::INTERNAL_SERVER_ERROR, "failed to retrieve created channel"),
    }
}

async fn get_channel(
    Path(channel_id): Path<String>,
    State(state): State<ApiState>,
) -> axum::response::Response {
    match state.store.get_channel(&channel_id) {
        Some(ch) => ApiResponse::ok(ChannelDto::from(ch)).into_response(),
        None     => err_resp(StatusCode::NOT_FOUND, format!("channel not found: {channel_id}")),
    }
}

async fn delete_channel(
    Path(channel_id): Path<String>,
    State(state): State<ApiState>,
    Extension(_identity): Extension<Identity>,
) -> axum::response::Response {
    if let Some(ch) = state.store.get_channel(&channel_id) {
        if ch.is_system {
            return err_resp(StatusCode::FORBIDDEN, "system channels cannot be deleted");
        }
    }
    match state.store.delete_channel(&channel_id) {
        Ok(()) => {
            emit(&state.events, "channel_deleted", json!({ "channel_id": channel_id }));
            ApiResponse::message(format!("channel {channel_id} deleted")).into_response()
        }
        Err(e) => err_resp(StatusCode::NOT_FOUND, e.to_string()),
    }
}

async fn get_channel_messages(
    Path(channel_id): Path<String>,
    Query(query): Query<HistoryQuery>,
    State(state): State<ApiState>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(50).min(200);
    let mut messages: Vec<MessageDto> = state
        .store
        .get_channel_messages(&channel_id, limit)
        .into_iter()
        .map(MessageDto::from)
        .collect();
    // Store returns newest-first; SDK expects oldest-first.
    messages.reverse();
    let has_more = messages.len() == limit;
    let oldest_id = messages.first().map(|m| m.id.clone());
    ApiResponse::ok(MessageHistoryDto { messages, has_more, oldest_id })
}

async fn subscribe_channel(
    Path(channel_id): Path<String>,
    State(state): State<ApiState>,
    Extension(identity): Extension<Identity>,
) -> axum::response::Response {
    if let Some(ch) = state.store.get_channel(&channel_id) {
        if ch.is_system && !identity.can_moderate() {
            return err_resp(StatusCode::FORBIDDEN, "system channel is admin/operator only");
        }
        if ch.frozen {
            return err_resp(StatusCode::FORBIDDEN, "channel is frozen");
        }
    }
    let client = ClientId::from_str(&identity.user_id);
    match state.store.subscribe_channel(&channel_id, &client) {
        Ok(()) => {
            emit(&state.events, "user_joined_channel", json!({
                "channel_id": channel_id,
                "user_id":    identity.user_id,
            }));
            ApiResponse::message(format!("subscribed to {channel_id}")).into_response()
        }
        Err(e) => err_resp(StatusCode::NOT_FOUND, e.to_string()),
    }
}

async fn unsubscribe_channel(
    Path(channel_id): Path<String>,
    State(state): State<ApiState>,
    Extension(identity): Extension<Identity>,
) -> axum::response::Response {
    let client = ClientId::from_str(&identity.user_id);
    match state.store.unsubscribe_channel(&channel_id, &client) {
        Ok(()) => {
            emit(&state.events, "user_left_channel", json!({
                "channel_id": channel_id,
                "user_id":    identity.user_id,
            }));
            ApiResponse::message(format!("unsubscribed from {channel_id}")).into_response()
        }
        Err(e) => err_resp(StatusCode::NOT_FOUND, e.to_string()),
    }
}

async fn send_typing(
    Path(channel_id): Path<String>,
    State(state): State<ApiState>,
    Extension(identity): Extension<Identity>,
) -> impl IntoResponse {
    emit(&state.events, "typing_started", json!({
        "channel_id": channel_id,
        "user_id":    identity.user_id,
        "username":   identity.user_id,
        "is_typing":  true,
    }));
    ApiResponse::message("typing forwarded")
}

async fn send_message(
    State(state): State<ApiState>,
    Extension(identity): Extension<Identity>,
    Json(req): Json<SendMessageRequest>,
) -> axum::response::Response {
    if req.text.trim().is_empty() {
        return err_resp(StatusCode::BAD_REQUEST, "text must not be empty");
    }
    match state.store.get_channel(&req.channel_id) {
        None => return err_resp(StatusCode::NOT_FOUND, format!("channel not found: {}", req.channel_id)),
        Some(ch) if ch.frozen => return err_resp(StatusCode::FORBIDDEN, "channel is frozen"),
        Some(_) => {}
    }
    let from = ClientId::from_str(&identity.user_id);
    let mut msg = Message::new(MessageType::Channel, from, req.channel_id, req.text);
    if let Some(reply_to) = req.reply_to_id {
        msg = msg.with_reply_to(reply_to);
    }
    state.store.store_message(msg.clone());
    let dto = MessageDto::from(msg);
    emit(&state.events, "message_sent", json!({ "message": &dto }));
    ApiResponse::ok(SendMessageResponseDto { message: dto }).into_response()
}

async fn get_message(
    Path(message_id): Path<String>,
    State(state): State<ApiState>,
) -> axum::response::Response {
    match state.store.get_message(&message_id) {
        Some(m) => ApiResponse::ok(MessageDto::from(m)).into_response(),
        None    => err_resp(StatusCode::NOT_FOUND, format!("message not found: {message_id}")),
    }
}

async fn delete_message(
    Path(message_id): Path<String>,
    State(state): State<ApiState>,
    Extension(_identity): Extension<Identity>,
) -> axum::response::Response {
    // Capture channel_id before deletion so we can emit it in the event.
    let channel_id = state
        .store
        .get_message(&message_id)
        .map(|m| m.to.clone());
    match state.store.delete_message(&message_id) {
        Ok(()) => {
            if let Some(cid) = channel_id {
                emit(&state.events, "message_deleted", json!({
                    "message_id": message_id,
                    "channel_id": cid,
                }));
            }
            ApiResponse::message(format!("message {message_id} deleted")).into_response()
        }
        Err(e) => err_resp(StatusCode::NOT_FOUND, e.to_string()),
    }
}

async fn edit_message(
    Path(message_id): Path<String>,
    State(state): State<ApiState>,
    Extension(_identity): Extension<Identity>,
    Json(req): Json<EditMessageRequest>,
) -> axum::response::Response {
    if req.text.trim().is_empty() {
        return err_resp(StatusCode::BAD_REQUEST, "text must not be empty");
    }
    match state.store.edit_message(&message_id, req.text) {
        Ok(m)  => ApiResponse::ok(MessageDto::from(m)).into_response(),
        Err(e) => err_resp(StatusCode::NOT_FOUND, e.to_string()),
    }
}

async fn add_reaction(
    Path(message_id): Path<String>,
    State(state): State<ApiState>,
    Extension(identity): Extension<Identity>,
    Json(req): Json<ReactionRequest>,
) -> axum::response::Response {
    match state.store.add_reaction(&message_id, req.emoji, identity.user_id) {
        Ok(())  => ApiResponse::message("reaction added").into_response(),
        Err(e) => err_resp(StatusCode::NOT_FOUND, e.to_string()),
    }
}

async fn freeze_channel(
    Path(channel_id): Path<String>,
    State(state): State<ApiState>,
    Extension(identity): Extension<Identity>,
) -> axum::response::Response {
    set_channel_frozen(&channel_id, true, &state, &identity)
}

async fn unfreeze_channel(
    Path(channel_id): Path<String>,
    State(state): State<ApiState>,
    Extension(identity): Extension<Identity>,
) -> axum::response::Response {
    set_channel_frozen(&channel_id, false, &state, &identity)
}

fn set_channel_frozen(
    channel_id: &str,
    frozen: bool,
    state: &ApiState,
    identity: &Identity,
) -> axum::response::Response {
    if !identity.is_admin() {
        return err_resp(StatusCode::FORBIDDEN, "admin role required");
    }
    match state.store.set_channel_frozen(&channel_id.to_string(), frozen) {
        Ok(()) => {
            emit(&state.events, "channel_frozen", json!({
                "channel_id": channel_id,
                "frozen":     frozen,
            }));
            ApiResponse::message(if frozen { "channel frozen" } else { "channel unfrozen" })
                .into_response()
        }
        Err(e) => err_resp(StatusCode::NOT_FOUND, e.to_string()),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_routes_non_empty_and_have_required_fields() {
        let routes = messaging_routes();
        assert!(!routes.is_empty());
        for r in &routes {
            assert!(!r.method.is_empty());
            assert!(r.path.starts_with('/'));
            assert!(!r.description.is_empty());
        }
    }

    #[test]
    fn test_expected_endpoints_present() {
        let routes = messaging_routes();
        let lookup: HashSet<(&str, &str)> = routes.iter().map(|r| (r.method, r.path)).collect();
        assert!(lookup.contains(&("GET",  "/api/channels")));
        assert!(lookup.contains(&("POST", "/api/channels")));
        assert!(lookup.contains(&("POST", "/api/messages/send")));
        assert!(lookup.contains(&("GET",  "/api/channels/{id}/messages")));
    }

    #[test]
    fn test_router_builds_with_store() {
        use server_core::event::EventBus;
        use tokio::sync::broadcast;

        struct NoopEvents;
        impl EventBusHandle for NoopEvents {
            fn publish(&self, _event: ServerEvent) {}
            fn subscribe(&self, _topic: &str) -> broadcast::Receiver<Arc<ServerEvent>> {
                broadcast::channel::<Arc<ServerEvent>>(1).0.subscribe()
            }
        }

        let _ = EventBus::new(8); // referenced so the import isn't dead
        let store = Arc::new(MessageStore::new(100));
        let events: Arc<dyn EventBusHandle> = Arc::new(NoopEvents);
        let _router: Router = router(store, events);
        // If it compiles + builds, the route table is well-formed.
    }
}
