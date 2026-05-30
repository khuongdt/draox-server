use crate::clan::Clan;
use crate::manager::ClanManager;
use axum::extract::{Path, State};
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

#[derive(Clone)]
struct ApiState {
    manager: Arc<ClanManager>,
    events:  Arc<dyn EventBusHandle>,
}

fn emit(events: &Arc<dyn EventBusHandle>, name: &str, payload: serde_json::Value) {
    events.publish(ServerEvent::Custom {
        source:  "clans".to_string(),
        name:    name.to_string(),
        payload,
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// DTOs
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ClanDto {
    pub id:          String,
    pub name:        String,
    pub tag:         String,
    pub description: String,
    pub owner_id:    String,
    pub member_count: usize,
    pub max_members: usize,
    pub created_at:  String,
    pub is_system:   bool,
    pub frozen:      bool,
}

impl From<Clan> for ClanDto {
    fn from(c: Clan) -> Self {
        Self {
            member_count: c.members.len(),
            id:           c.id,
            name:         c.name,
            tag:          c.tag,
            description:  c.description,
            owner_id:     c.owner_id.as_str().to_string(),
            max_members:  c.max_members,
            created_at:   c.created_at.to_rfc3339(),
            is_system:    c.is_system,
            frozen:       c.frozen,
        }
    }
}

#[derive(Serialize)]
pub struct ClanMemberDto {
    pub client_id: String,
    pub role:      &'static str,
    pub joined_at: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Response wrapper
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
    let body = json!({
        "success": false,
        "error":   status.canonical_reason().unwrap_or("Unknown"),
        "message": msg.into(),
    });
    (status, Json(body)).into_response()
}

// ─────────────────────────────────────────────────────────────────────────────
// Request bodies
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateClanRequest {
    pub name: String,
    pub tag:  String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Router
// ─────────────────────────────────────────────────────────────────────────────

pub fn router(manager: Arc<ClanManager>, events: Arc<dyn EventBusHandle>) -> Router {
    let state = ApiState { manager, events };
    Router::new()
        .route("/api/clans",                  get(list_clans).post(create_clan))
        .route("/api/clans/{id}",             get(get_clan).delete(delete_clan))
        .route("/api/clans/{id}/join",        post(join_clan))
        .route("/api/clans/{id}/leave",       post(leave_clan))
        .route("/api/clans/{id}/freeze",      post(freeze_clan))
        .route("/api/clans/{id}/unfreeze",    post(unfreeze_clan))
        .route("/api/clans/{id}/members",     get(list_members))
        .route("/api/clans/{id}/stats",       get(clan_stats))
        .with_state(state)
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

async fn list_clans(State(state): State<ApiState>) -> impl IntoResponse {
    let clans: Vec<ClanDto> = state.manager.list_clans().into_iter().map(ClanDto::from).collect();
    ApiResponse::ok(clans)
}

async fn create_clan(
    State(state): State<ApiState>,
    Extension(identity): Extension<Identity>,
    Json(req): Json<CreateClanRequest>,
) -> axum::response::Response {
    if req.name.trim().is_empty() || req.tag.trim().is_empty() {
        return err_resp(StatusCode::BAD_REQUEST, "name and tag must not be empty");
    }
    let owner = ClientId::from_str(&identity.user_id);
    match state.manager.create_clan(req.name, req.tag, owner) {
        Ok(id) => match state.manager.get_clan(&id) {
            Ok(clan) => {
                let dto = ClanDto::from(clan);
                emit(&state.events, "clan_created", json!({ "clan": &dto }));
                ApiResponse::ok(dto).into_response()
            }
            Err(e) => err_resp(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        },
        Err(e) => err_resp(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

async fn get_clan(
    Path(clan_id): Path<String>,
    State(state): State<ApiState>,
) -> axum::response::Response {
    match state.manager.get_clan(&clan_id) {
        Ok(clan) => ApiResponse::ok(ClanDto::from(clan)).into_response(),
        Err(_)   => err_resp(StatusCode::NOT_FOUND, format!("clan not found: {clan_id}")),
    }
}

async fn delete_clan(
    Path(clan_id): Path<String>,
    State(state): State<ApiState>,
    Extension(identity): Extension<Identity>,
) -> axum::response::Response {
    if let Ok(clan) = state.manager.get_clan(&clan_id) {
        if clan.is_system {
            return err_resp(StatusCode::FORBIDDEN, "system clans cannot be deleted");
        }
    }
    let requester = ClientId::from_str(&identity.user_id);
    match state.manager.delete_clan(&clan_id, &requester) {
        Ok(()) => {
            emit(&state.events, "clan_deleted", json!({ "clan_id": clan_id }));
            ApiResponse::message(format!("clan {clan_id} deleted")).into_response()
        }
        Err(e) => err_resp(StatusCode::FORBIDDEN, e.to_string()),
    }
}

async fn join_clan(
    Path(clan_id): Path<String>,
    State(state): State<ApiState>,
    Extension(identity): Extension<Identity>,
) -> axum::response::Response {
    if let Ok(clan) = state.manager.get_clan(&clan_id) {
        if clan.is_system && !identity.can_moderate() {
            return err_resp(StatusCode::FORBIDDEN, "system clan is admin/operator only");
        }
        if clan.frozen {
            return err_resp(StatusCode::FORBIDDEN, "clan is frozen");
        }
    }
    let client = ClientId::from_str(&identity.user_id);
    match state.manager.join_clan(&clan_id, client) {
        Ok(()) => {
            emit(&state.events, "member_joined", json!({
                "clan_id": clan_id,
                "user_id": identity.user_id,
            }));
            ApiResponse::message(format!("joined {clan_id}")).into_response()
        }
        Err(e) => err_resp(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

async fn freeze_clan(
    Path(clan_id): Path<String>,
    State(state): State<ApiState>,
    Extension(identity): Extension<Identity>,
) -> axum::response::Response {
    set_clan_frozen(&clan_id, true, &state, &identity)
}

async fn unfreeze_clan(
    Path(clan_id): Path<String>,
    State(state): State<ApiState>,
    Extension(identity): Extension<Identity>,
) -> axum::response::Response {
    set_clan_frozen(&clan_id, false, &state, &identity)
}

fn set_clan_frozen(
    clan_id: &str,
    frozen: bool,
    state: &ApiState,
    identity: &Identity,
) -> axum::response::Response {
    if !identity.is_admin() {
        return err_resp(StatusCode::FORBIDDEN, "admin role required");
    }
    match state.manager.set_clan_frozen(&clan_id.to_string(), frozen) {
        Ok(()) => {
            emit(&state.events, "clan_frozen", json!({
                "clan_id": clan_id,
                "frozen":  frozen,
            }));
            ApiResponse::message(if frozen { "clan frozen" } else { "clan unfrozen" })
                .into_response()
        }
        Err(e) => err_resp(StatusCode::NOT_FOUND, e.to_string()),
    }
}

async fn leave_clan(
    Path(clan_id): Path<String>,
    State(state): State<ApiState>,
    Extension(identity): Extension<Identity>,
) -> axum::response::Response {
    let client = ClientId::from_str(&identity.user_id);
    match state.manager.leave_clan(&clan_id, &client) {
        Ok(()) => {
            emit(&state.events, "member_left", json!({
                "clan_id": clan_id,
                "user_id": identity.user_id,
            }));
            ApiResponse::message(format!("left {clan_id}")).into_response()
        }
        Err(e) => err_resp(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

async fn list_members(
    Path(clan_id): Path<String>,
    State(state): State<ApiState>,
) -> axum::response::Response {
    match state.manager.get_clan(&clan_id) {
        Ok(clan) => {
            let members: Vec<ClanMemberDto> = clan.members.into_iter().map(|m| ClanMemberDto {
                client_id: m.client_id.as_str().to_string(),
                role: match m.role {
                    crate::clan::ClanRole::Owner   => "Owner",
                    crate::clan::ClanRole::Officer => "Officer",
                    crate::clan::ClanRole::Member  => "Member",
                    crate::clan::ClanRole::Recruit => "Recruit",
                },
                joined_at: m.joined_at.to_rfc3339(),
            }).collect();
            ApiResponse::ok(members).into_response()
        }
        Err(_) => err_resp(StatusCode::NOT_FOUND, format!("clan not found: {clan_id}")),
    }
}

async fn clan_stats(
    Path(clan_id): Path<String>,
    State(state): State<ApiState>,
) -> axum::response::Response {
    match state.manager.get_stats(&clan_id) {
        Ok(stats) => ApiResponse::ok(stats).into_response(),
        Err(_)    => err_resp(StatusCode::NOT_FOUND, format!("clan not found: {clan_id}")),
    }
}
