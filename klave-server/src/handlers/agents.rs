use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tracing::{error, info};

use klave_core::agent::model::{AgentBalance, AgentPolicyInput, CreateAgentRequest};

use crate::response::ApiResponse;
use crate::state::AppState;

pub async fn create_agent(
    State(state): State<AppState>,
    Json(body): Json<CreateAgentRequest>,
) -> Response {
    if body.label.trim().is_empty() {
        return ApiResponse::<()>::error(StatusCode::BAD_REQUEST, "label must not be empty")
            .into_response();
    }

    match state.agent_repo.create(&body).await {
        Ok(agent) => {
            info!(agent_id = %agent.id, pubkey = %agent.pubkey, "agent created");
            ApiResponse::created(
                serde_json::to_value(&agent).expect("agent serialization"),
                "agent created",
            )
            .into_response()
        }
        Err(e) => {
            error!(error = %e, "failed to create agent");
            ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                .into_response()
        }
    }
}

pub async fn list_agents(State(state): State<AppState>) -> Response {
    match state.agent_repo.list_all().await {
        Ok(agents) => ApiResponse::success(
            serde_json::to_value(&agents).expect("agents serialization"),
            "agents retrieved",
        )
        .into_response(),
        Err(e) => {
            error!(error = %e, "failed to list agents");
            ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                .into_response()
        }
    }
}

pub async fn deactivate_agent(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.agent_repo.deactivate(&id).await {
        Ok(()) => {
            info!(agent_id = %id, "agent deactivated");
            ApiResponse::<()>::no_content("agent deactivated").into_response()
        }
        Err(klave_core::error::KlaveError::AgentNotFound(_)) => {
            ApiResponse::<()>::error(StatusCode::NOT_FOUND, format!("agent not found: {id}"))
                .into_response()
        }
        Err(e) => {
            error!(error = %e, "failed to deactivate agent");
            ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                .into_response()
        }
    }
}

pub async fn get_agent_history(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let agent = match state.agent_repo.find_by_id(&id).await {
        Ok(Some(a)) => a,
        Ok(None) => {
            return ApiResponse::<()>::error(
                StatusCode::NOT_FOUND,
                format!("agent not found: {id}"),
            )
            .into_response();
        }
        Err(e) => {
            error!(error = %e, "failed to find agent");
            return ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                .into_response();
        }
    };

    match state.audit_store.list_by_agent(&agent.id).await {
        Ok(entries) => ApiResponse::success(
            serde_json::to_value(&entries).expect("audit entries serialization"),
            "transaction history retrieved",
        )
        .into_response(),
        Err(e) => {
            error!(error = %e, "failed to retrieve audit log");
            ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                .into_response()
        }
    }
}

pub async fn get_agent_balance(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.agent_repo.find_by_id(&id).await {
        Ok(Some(_)) => {
            // Phase 1 stub. Solana RPC balance queries are wired in Phase 2.
            let balance = AgentBalance {
                sol_lamports: 0,
                vault_lamports: 0,
            };
            ApiResponse::success(
                serde_json::to_value(&balance).expect("balance serialization"),
                "balance retrieved (stub: Solana RPC integration pending)",
            )
            .into_response()
        }
        Ok(None) => {
            ApiResponse::<()>::error(StatusCode::NOT_FOUND, format!("agent not found: {id}"))
                .into_response()
        }
        Err(e) => {
            error!(error = %e, "failed to find agent");
            ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                .into_response()
        }
    }
}

pub async fn update_policy(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<AgentPolicyInput>,
) -> Response {
    match state.agent_repo.find_by_id(&id).await {
        Ok(Some(agent)) => {
            if !agent.is_active {
                return ApiResponse::<()>::error(
                    StatusCode::BAD_REQUEST,
                    format!("cannot update policy for inactive agent: {id}"),
                )
                .into_response();
            }
        }
        Ok(None) => {
            return ApiResponse::<()>::error(
                StatusCode::NOT_FOUND,
                format!("agent not found: {id}"),
            )
            .into_response();
        }
        Err(e) => {
            error!(error = %e, "failed to find agent");
            return ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                .into_response();
        }
    }

    match state.agent_repo.update_policy(&id, &body).await {
        Ok(policy) => {
            info!(agent_id = %id, "policy updated");
            ApiResponse::success(
                serde_json::to_value(&policy).expect("policy serialization"),
                "policy updated",
            )
            .into_response()
        }
        Err(e) => {
            error!(error = %e, "failed to update policy");
            ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                .into_response()
        }
    }
}
