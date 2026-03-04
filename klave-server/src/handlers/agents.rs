use std::str::FromStr;
use uuid::Uuid;

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use klave_core::{
    agent::model::{AgentBalance, AgentPolicyInput, CreateAgentRequest},
    audit::store::NewAuditEntry,
    error::KlaveError,
};
use solana_sdk::pubkey::Pubkey;
use tracing::{error, info};

use crate::{event::ServerEvent, middleware::AuthContext, response::ApiResponse, state::AppState};

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
            info!(agent_id = %agent.id, pubkey = %agent.pubkey, label = %agent.label, "agent wallet created");

            let _ = state.event_tx.send(ServerEvent::AgentCreated {
                id: agent.id.clone(),
                label: agent.label.clone(),
            });

            let entry = NewAuditEntry {
                agent_id: agent.id.clone(),
                instruction_type: "wallet_created".to_string(),
                status: "confirmed".to_string(),
                tx_signature: None,
                policy_violations: None,
                metadata: Some(serde_json::json!({
                    "pubkey": agent.pubkey,
                    "label": agent.label,
                })),
            };
            let _ = state.audit_store.append(&entry).await;

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

pub async fn list_agents(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Response {
    if auth.is_operator {
        match state.agent_repo.list_all().await {
            Ok(agents) => match serde_json::to_value(&agents) {
                Ok(val) => ApiResponse::success(val, "agents retrieved").into_response(),
                Err(e) => {
                    ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                        .into_response()
                }
            },
            Err(e) => {
                error!(error = %e, "failed to list agents");
                ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                    .into_response()
            }
        }
    } else if let Some(agent_id) = auth.agent_id {
        match state.agent_repo.find_by_id(&agent_id).await {
            Ok(Some(agent)) => match serde_json::to_value(vec![agent]) {
                Ok(val) => ApiResponse::success(val, "agent retrieved").into_response(),
                Err(e) => {
                    ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                        .into_response()
                }
            },
            _ => ApiResponse::<()>::error(StatusCode::NOT_FOUND, "Agent not found").into_response(),
        }
    } else {
        ApiResponse::<()>::error(StatusCode::UNAUTHORIZED, "Unauthorized").into_response()
    }
}

pub async fn deactivate_agent(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Response {
    if !auth.is_operator && auth.agent_id.as_deref() != Some(&id) {
        return ApiResponse::<()>::error(StatusCode::FORBIDDEN, "Forbidden").into_response();
    }

    if Uuid::parse_str(&id).is_err() {
        return ApiResponse::<()>::error(StatusCode::BAD_REQUEST, "Invalid Agent ID format")
            .into_response();
    }
    match state.agent_repo.deactivate(&id).await {
        Ok(()) => {
            info!(agent_id = %id, "agent deactivated");
            ApiResponse::<()>::no_content("agent deactivated").into_response()
        }
        Err(KlaveError::AgentNotFound(_)) => {
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

pub async fn get_agent_history(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Response {
    if !auth.is_operator && auth.agent_id.as_deref() != Some(&id) {
        return ApiResponse::<()>::error(StatusCode::FORBIDDEN, "Forbidden").into_response();
    }

    if Uuid::parse_str(&id).is_err() {
        return ApiResponse::<()>::error(StatusCode::BAD_REQUEST, "Invalid Agent ID format")
            .into_response();
    }
    let agent = match state.agent_repo.find_by_id(&id).await {
        Ok(Some(agent)) => agent,
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

pub async fn get_agent_balance(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Response {
    if !auth.is_operator && auth.agent_id.as_deref() != Some(&id) {
        return ApiResponse::<()>::error(StatusCode::FORBIDDEN, "Forbidden").into_response();
    }

    if Uuid::parse_str(&id).is_err() {
        return ApiResponse::<()>::error(StatusCode::BAD_REQUEST, "Invalid Agent ID format")
            .into_response();
    }
    let agent = match state.agent_repo.find_by_id(&id).await {
        Ok(Some(agent)) => agent,
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

    let agent_pubkey: Pubkey = match FromStr::from_str(&agent.pubkey) {
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
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
    Json(body): Json<AgentPolicyInput>,
) -> Response {
    if !auth.is_operator && auth.agent_id.as_deref() != Some(&id) {
        return ApiResponse::<()>::error(StatusCode::FORBIDDEN, "Forbidden").into_response();
    }

    if Uuid::parse_str(&id).is_err() {
        return ApiResponse::<()>::error(StatusCode::BAD_REQUEST, "Invalid Agent ID format")
            .into_response();
    }
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
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Response {
    if !auth.is_operator && auth.agent_id.as_deref() != Some(&id) {
        return ApiResponse::<()>::error(StatusCode::FORBIDDEN, "Forbidden").into_response();
    }

    if Uuid::parse_str(&id).is_err() {
        return ApiResponse::<()>::error(StatusCode::BAD_REQUEST, "Invalid Agent ID format")
            .into_response();
    }
    let agent = match state.agent_repo.find_by_id(&id).await {
        Ok(Some(agent)) => agent,
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

    let agent_pubkey: Pubkey = match FromStr::from_str(&agent.pubkey) {
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

pub async fn notify_balance_updated(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Response {
    if !auth.is_operator && auth.agent_id.as_deref() != Some(&id) {
        return ApiResponse::<()>::error(StatusCode::FORBIDDEN, "Forbidden").into_response();
    }

    if Uuid::parse_str(&id).is_err() {
        return ApiResponse::<()>::error(StatusCode::BAD_REQUEST, "Invalid Agent ID format")
            .into_response();
    }

    let agent = match state.agent_repo.find_by_id(&id).await {
        Ok(Some(agent)) => agent,
        Ok(None) => {
            return ApiResponse::<()>::error(StatusCode::NOT_FOUND, "agent not found")
                .into_response();
        }
        Err(e) => {
            return ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                .into_response();
        }
    };

    let agent_pubkey: Pubkey = match FromStr::from_str(&agent.pubkey) {
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

    let (sol, vault) = state
        .kora_gateway
        .get_balances(&agent_pubkey, &vault_pda)
        .await
        .unwrap_or((0, 0));
    let tokens = state
        .kora_gateway
        .get_token_balances(&agent_pubkey)
        .await
        .unwrap_or_default();

    let _ = state.event_tx.send(ServerEvent::BalanceUpdated {
        agent_id: id.clone(),
        sol_lamports: sol,
        vault_lamports: vault,
        tokens,
    });
    info!(agent_id = %id, "balance update notification sent");

    ApiResponse::<()>::no_content("balance update notification sent").into_response()
}
