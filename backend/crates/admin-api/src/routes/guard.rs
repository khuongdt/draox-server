use crate::error::ApiError;
use crate::response::ApiResponse;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Serialize)]
pub struct GuardStatsResponse {
    pub active_bans: usize,
    pub blacklisted_entries: usize,
    pub whitelisted_entries: usize,
}

#[derive(Deserialize)]
pub struct BanRequest {
    pub ip: String,
    pub reason: Option<String>,
}

#[derive(Deserialize)]
pub struct UnbanRequest {
    pub ip: String,
}

/// GET /api/guard/stats
pub async fn guard_stats(State(state): State<AppState>) -> impl IntoResponse {
    let ban_mgr = state.traffic_guard.ban_manager();
    let ip_filter = state.traffic_guard.ip_filter();

    ApiResponse::ok(GuardStatsResponse {
        active_bans: ban_mgr.active_ban_count(),
        blacklisted_entries: ip_filter.blacklist_count(),
        whitelisted_entries: ip_filter.whitelist_count(),
    })
}

/// POST /api/guard/ban
pub async fn ban_ip(
    State(state): State<AppState>,
    Json(req): Json<BanRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let ip: IpAddr = req
        .ip
        .parse()
        .map_err(|_| ApiError::bad_request(format!("invalid IP address: {}", req.ip)))?;

    let ban_mgr = state.traffic_guard.ban_manager();
    let reason = req.reason.as_deref().unwrap_or("admin API ban");
    let entry = ban_mgr.ban(ip, reason);

    Ok(ApiResponse::<()>::message(format!(
        "IP {ip} banned (ban #{}, expires {})",
        entry.ban_count,
        entry.expires_at.to_rfc3339()
    )))
}

/// POST /api/guard/unban
pub async fn unban_ip(
    State(state): State<AppState>,
    Json(req): Json<UnbanRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let ip: IpAddr = req
        .ip
        .parse()
        .map_err(|_| ApiError::bad_request(format!("invalid IP address: {}", req.ip)))?;

    let ban_mgr = state.traffic_guard.ban_manager();
    let removed = ban_mgr.unban(ip);

    if removed {
        Ok(ApiResponse::<()>::message(format!("IP {ip} unbanned")))
    } else {
        Err(ApiError::not_found(format!("IP {ip} is not banned")))
    }
}

/// GET /api/guard/bans — list all active bans
#[derive(Serialize)]
pub struct BanListResponse {
    pub total: usize,
    pub bans: Vec<BanInfoResponse>,
}

#[derive(Serialize)]
pub struct BanInfoResponse {
    pub ip: String,
    pub reason: String,
    pub expires_at: String,
    pub ban_count: u32,
}

pub async fn list_bans(State(state): State<AppState>) -> impl IntoResponse {
    let ban_mgr = state.traffic_guard.ban_manager();
    let bans: Vec<BanInfoResponse> = ban_mgr
        .active_bans()
        .iter()
        .map(|entry| BanInfoResponse {
            ip: entry.key().to_string(),
            reason: entry.value().reason.clone(),
            expires_at: entry.value().expires_at.to_rfc3339(),
            ban_count: entry.value().ban_count,
        })
        .collect();
    let total = bans.len();
    ApiResponse::ok(BanListResponse { total, bans })
}

/// POST /api/guard/whitelist — add IP to whitelist
#[derive(Deserialize)]
pub struct WhitelistRequest {
    pub ip: String,
}

pub async fn add_whitelist(
    State(state): State<AppState>,
    Json(req): Json<WhitelistRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate the IP/CIDR by attempting to parse it
    let _: IpAddr = req
        .ip
        .parse()
        .map_err(|_| ApiError::bad_request(format!("invalid IP: {}", req.ip)))?;
    state
        .traffic_guard
        .ip_filter()
        .add_whitelist(&req.ip)
        .map_err(|e| ApiError::bad_request(e))?;
    Ok(ApiResponse::<()>::message(format!(
        "IP {} added to whitelist",
        req.ip
    )))
}

/// POST /api/guard/blacklist — add IP to blacklist
#[derive(Deserialize)]
pub struct BlacklistRequest {
    pub ip: String,
}

pub async fn add_blacklist(
    State(state): State<AppState>,
    Json(req): Json<BlacklistRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate the IP/CIDR by attempting to parse it
    let _: IpAddr = req
        .ip
        .parse()
        .map_err(|_| ApiError::bad_request(format!("invalid IP: {}", req.ip)))?;
    state
        .traffic_guard
        .ip_filter()
        .add_blacklist(&req.ip)
        .map_err(|e| ApiError::bad_request(e))?;
    Ok(ApiResponse::<()>::message(format!(
        "IP {} added to blacklist",
        req.ip
    )))
}

/// GET /api/guard/reputation/:ip — get reputation score for an IP
pub async fn get_reputation(
    State(state): State<AppState>,
    Path(ip_str): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let ip: IpAddr = ip_str
        .parse()
        .map_err(|_| ApiError::bad_request(format!("invalid IP: {ip_str}")))?;
    let score = state.traffic_guard.reputation().get_score(ip);
    Ok(ApiResponse::ok(serde_json::json!({
        "ip": ip_str,
        "score": score,
    })))
}
