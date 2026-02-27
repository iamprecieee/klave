use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use solana_sdk::pubkey::Pubkey;
use tracing::{error, info};

use klave_core::agent::model::{AgentBalance, AgentPolicyInput, CreateAgentRequest};

use crate::{response::ApiResponse, state::AppState};

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
            match serde_json::to_value(&agent) {
                Ok(val) => ApiResponse::created(val, "agent created").into_response(),
                Err(e) => {
                    ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                        .into_response()
                }
            }
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
        Ok(agents) => match serde_json::to_value(&agents) {
            Ok(val) => ApiResponse::success(val, "agents retrieved").into_response(),
            Err(e) => ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                .into_response(),
        },
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
        Ok(entries) => match serde_json::to_value(&entries) {
            Ok(val) => ApiResponse::success(val, "transaction history retrieved").into_response(),
            Err(e) => ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                .into_response(),
        },
        Err(e) => {
            error!(error = %e, "failed to retrieve audit log");
            ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                .into_response()
        }
    }
}

pub async fn get_agent_balance(State(state): State<AppState>, Path(id): Path<String>) -> Response {
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

    let agent_pubkey: Pubkey = match std::str::FromStr::from_str(&agent.pubkey) {
        Ok(pk) => pk,
        Err(_) => {
            return ApiResponse::<()>::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Invalid pubkey".to_string(),
            )
            .into_response();
        }
    };

    let program_id = Pubkey::new_from_array(klave_anchor::ID.to_bytes());
    let (vault_pda, _) =
        Pubkey::find_program_address(&[b"vault", agent_pubkey.as_ref()], &program_id);

    match state
        .kora_gateway
        .get_balances(&agent_pubkey, &vault_pda)
        .await
    {
        Ok((sol_lamports, vault_lamports)) => {
            let balance = AgentBalance {
                sol_lamports,
                vault_lamports,
            };
            match serde_json::to_value(&balance) {
                Ok(val) => ApiResponse::success(val, "agent balance retrieved").into_response(),
                Err(e) => {
                    ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                        .into_response()
                }
            }
        }
        Err(e) => {
            error!(error = %e, "failed to fetch balances");
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
            match serde_json::to_value(&policy) {
                Ok(val) => ApiResponse::success(val, "policy updated").into_response(),
                Err(e) => {
                    ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                        .into_response()
                }
            }
        }
        Err(e) => {
            error!(error = %e, "failed to update policy");
            ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                .into_response()
        }
    }
}

pub async fn get_agent_token_balances(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
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

    let agent_pubkey: Pubkey = match std::str::FromStr::from_str(&agent.pubkey) {
        Ok(pk) => pk,
        Err(_) => {
            return ApiResponse::<()>::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Invalid pubkey".to_string(),
            )
            .into_response();
        }
    };

    match state.kora_gateway.get_token_balances(&agent_pubkey).await {
        Ok(balances) => match serde_json::to_value(&balances) {
            Ok(val) => ApiResponse::success(val, "agent token balances retrieved").into_response(),
            Err(e) => ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                .into_response(),
        },
        Err(e) => {
            error!(error = %e, "failed to fetch token balances");
            ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                .into_response()
        }
    }
}
