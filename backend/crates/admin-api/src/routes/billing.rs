use crate::response::ApiResponse;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;
use billing::plans::{Plan, PlanTier};
use server_core::ClientId;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct BillingUsageResponse {
    pub client_id: String,
    pub plan: PlanTier,
    pub requests: u64,
    pub bandwidth_bytes: u64,
}

/// GET /api/billing/usage/:client_id
pub async fn get_usage(
    State(state): State<AppState>,
    Path(client_id_str): Path<String>,
) -> impl IntoResponse {
    let client_id = ClientId::from_str(&client_id_str);
    let usage = state.usage_tracker.get_usage(&client_id);
    ApiResponse::ok(BillingUsageResponse {
        client_id: client_id_str,
        plan: usage.plan,
        requests: usage.requests,
        bandwidth_bytes: usage.bandwidth_bytes,
    })
}

/// GET /api/billing/plans — list all available plans
pub async fn list_plans() -> impl IntoResponse {
    let plans = vec![Plan::free(), Plan::pro(), Plan::enterprise()];
    ApiResponse::ok(plans)
}

#[derive(Deserialize)]
pub struct SetPlanRequest {
    pub plan: PlanTier,
}

/// PUT /api/billing/plan/:client_id — set a client's plan
pub async fn set_plan(
    State(state): State<AppState>,
    Path(client_id_str): Path<String>,
    Json(req): Json<SetPlanRequest>,
) -> impl IntoResponse {
    let client_id = ClientId::from_str(&client_id_str);
    state.usage_tracker.set_plan(&client_id, req.plan);
    ApiResponse::<()>::message(format!("plan set to {:?} for {client_id_str}", req.plan))
}
