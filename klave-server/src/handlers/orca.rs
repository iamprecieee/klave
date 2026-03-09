use std::{str::FromStr, sync::Arc};
use uuid::Uuid;

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use klave_core::{
    agent::model::{Agent, AgentPolicy, SwapQuote},
    audit::store::NewAuditEntry,
    policy::engine::{InstructionType, PolicyEngine},
};
use orca_whirlpools::SwapType;
use serde::{Deserialize, Serialize};
use solana_keychain::SolanaSigner;
use solana_sdk::{
    message::Message,
    pubkey::Pubkey,
    signature::{Signature, Signer},
    transaction::{Transaction, VersionedTransaction},
};

use crate::{event::ServerEvent, middleware::AuthContext, response::ApiResponse, state::AppState};

#[derive(Debug, Deserialize)]
pub struct OrcaSwapRequest {
    pub whirlpool: String,
    pub input_mint: String,
    pub output_mint: Option<String>,
    pub amount: u64,
    pub slippage_bps: Option<u16>,
}

#[derive(Debug, Serialize)]
pub struct OrcaSwapResponse {
    pub tx_signature: String,
    pub via_kora: bool,
}

#[derive(Debug, Deserialize)]
pub struct PoolsQuery {
    pub token: Option<String>,
    pub limit: Option<usize>,
}

pub async fn execute_swap(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
    Json(payload): Json<OrcaSwapRequest>,
) -> Response {
    if !auth.is_operator && auth.agent_id.as_deref() != Some(&id) {
        return ApiResponse::<()>::error(StatusCode::FORBIDDEN, "Forbidden").into_response();
    }

    let agent_id = match Uuid::from_str(&id) {
        Ok(uuid) => uuid.to_string(),
        Err(_) => {
            return ApiResponse::<()>::error(StatusCode::BAD_REQUEST, "invalid agent id format")
                .into_response();
        }
    };

    let agent = match state.agent_repo.find_by_id(&agent_id).await {
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

    let policy = match state.agent_repo.find_policy(&agent_id).await {
        Ok(Some(policy)) => policy,
        Ok(None) => {
            return ApiResponse::<()>::error(StatusCode::NOT_FOUND, "agent policy not found")
                .into_response();
        }
        Err(e) => {
            return ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                .into_response();
        }
    };

    let agent_pubkey = match Pubkey::from_str(&agent.pubkey) {
        Ok(pk) => pk,
        Err(_) => {
            return ApiResponse::<()>::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "invalid agent pubkey".to_string(),
            )
            .into_response();
        }
    };

    let whirlpool = match Pubkey::from_str(&payload.whirlpool) {
        Ok(pool) => pool,
        Err(_) => {
            return ApiResponse::<()>::error(StatusCode::BAD_REQUEST, "invalid whirlpool address")
                .into_response();
        }
    };
    let input_mint = match Pubkey::from_str(&payload.input_mint) {
        Ok(mint) => mint,
        Err(_) => {
            return ApiResponse::<()>::error(StatusCode::BAD_REQUEST, "invalid input mint address")
                .into_response();
        }
    };
    let slippage_bps = payload.slippage_bps.unwrap_or(policy.slippage_bps as u16);

    let lock_arc = state
        .agent_locks
        .entry(agent.id.clone())
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone();
    let _agent_lock = lock_arc.lock().await;

    let swap_usd_value =
        match enforce_policies(&state, &agent, &policy, &payload, slippage_bps).await {
            Ok(val) => val,
            Err(resp) => return resp,
        };

    let versioned_tx = match build_and_sign_tx(
        &state,
        &agent_id,
        agent_pubkey,
        whirlpool,
        input_mint,
        payload.amount,
        slippage_bps,
    )
    .await
    {
        Ok(tx) => tx,
        Err(resp) => return resp,
    };

    let (tx_signature, via_kora) = match state.kora_gateway.send_transaction(&versioned_tx).await {
        Ok(res) => res,
        Err(e) => {
            tracing::error!("Kora transaction error for agent {}: {}", agent.id, e);
            return ApiResponse::<()>::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Transaction failed: {}", e),
            )
            .into_response();
        }
    };

    let tx_sig_str = tx_signature.to_string();
    write_confirmed_audit(
        &state,
        &agent.id,
        &tx_sig_str,
        &payload,
        slippage_bps,
        swap_usd_value,
    )
    .await;

    tracing::info!(
        agent_id = %agent.id,
        tx_signature = %tx_sig_str,
        input_mint = %payload.input_mint,
        amount = %payload.amount,
        via_kora = %via_kora,
        "orca swap executed"
    );

    let _ = state.event_tx.send(ServerEvent::TransactionExecuted {
        signature: tx_sig_str.clone(),
        agent_id: agent.id.clone(),
    });

    spawn_orca_confirmation_task(state.clone(), agent_id, agent_pubkey, tx_signature);

    ApiResponse::success(
        OrcaSwapResponse {
            tx_signature: tx_sig_str,
            via_kora,
        },
        "orca swap executed successfully",
    )
    .into_response()
}

pub async fn get_swap_quote(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
    Json(payload): Json<OrcaSwapRequest>,
) -> ApiResponse<SwapQuote> {
    if !auth.is_operator && auth.agent_id.as_deref() != Some(&id) {
        return ApiResponse::error(StatusCode::FORBIDDEN, "Forbidden");
    }

    let whirlpool = match Pubkey::from_str(&payload.whirlpool) {
        Ok(pool) => pool,
        Err(_) => return ApiResponse::error(StatusCode::BAD_REQUEST, "invalid whirlpool address"),
    };
    let input_mint = match Pubkey::from_str(&payload.input_mint) {
        Ok(mint) => mint,
        Err(_) => return ApiResponse::error(StatusCode::BAD_REQUEST, "invalid input mint address"),
    };

    let agent = match state.agent_repo.find_by_id(&id).await {
        Ok(Some(agent)) => agent,
        Ok(None) => return ApiResponse::error(StatusCode::NOT_FOUND, "agent not found"),
        Err(e) => return ApiResponse::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let agent_pubkey = match Pubkey::from_str(&agent.pubkey) {
        Ok(pk) => pk,
        Err(e) => {
            tracing::error!("Failed to parse agent pubkey {}: {}", agent.pubkey, e);
            return ApiResponse::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "invalid agent pubkey in database",
            );
        }
    };

    let slippage_bps = payload.slippage_bps.unwrap_or(50);

    match state
        .orca_client
        .fetch_quote(
            whirlpool,
            payload.amount,
            input_mint,
            SwapType::ExactIn,
            slippage_bps,
            Some(agent_pubkey),
        )
        .await
    {
        Ok(quote) => ApiResponse::success(quote, "swap quote retrieved"),
        Err(e) => {
            tracing::error!("Orca quote error for agent {}: {}", id, e);
            ApiResponse::error(StatusCode::BAD_GATEWAY, e.to_string())
        }
    }
}

pub async fn get_orca_pools(
    State(state): State<AppState>,
    Query(query): Query<PoolsQuery>,
) -> ApiResponse<serde_json::Value> {
    match state.orca_client.list_pools(query.token, query.limit).await {
        Ok(data) => ApiResponse::success(data, "devnet orca pools retrieved successfully"),
        Err(e) => {
            tracing::error!("Failed to retrieve Orca pools: {}", e);
            ApiResponse::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
    }
}

async fn enforce_policies(
    state: &AppState,
    agent: &Agent,
    policy: &AgentPolicy,
    payload: &OrcaSwapRequest,
    slippage_bps: u16,
) -> Result<f64, Response> {
    let output_mint = payload
        .output_mint
        .as_deref()
        .unwrap_or(&payload.input_mint);

    // Check static token allowlist + slippage
    if let Err(violations) = PolicyEngine::check_swap_static(
        policy,
        &payload.input_mint,
        output_mint,
        slippage_bps as i32,
    ) {
        let violations: Vec<String> = violations.iter().map(|val| val.to_string()).collect();
        write_rejected_audit(state, &agent.id, violations.clone()).await;
        return Err(ApiResponse::<()>::error(
            StatusCode::FORBIDDEN,
            format!("Policy Violations: {:?}", violations),
        )
        .into_response());
    }

    // Check dynamic volume limits
    let swap_usd_value = state.price_feed.lamports_to_usd(payload.amount).await;

    if policy.daily_swap_volume_usd > 0.0 {
        if swap_usd_value == 0.0 {
            tracing::warn!("price feed unavailable, blocking swap to enforce volume policy");
            return Err(ApiResponse::<()>::error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Swap volume policy cannot be enforced: price feed unavailable".to_string(),
            )
            .into_response());
        }

        let daily_swap_volume =
            state
                .audit_store
                .sum_swap_volume(&agent.id)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "failed to fetch swap volume");
                    ApiResponse::<()>::error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to assess swap volume limit".to_string(),
                    )
                    .into_response()
                })?;

        if let Err(violations) =
            PolicyEngine::check_swap_volume(policy, swap_usd_value, daily_swap_volume)
        {
            let violations: Vec<String> = violations.iter().map(|val| val.to_string()).collect();
            write_rejected_audit(state, &agent.id, violations.clone()).await;
            return Err(ApiResponse::<()>::error(
                StatusCode::FORBIDDEN,
                format!("Policy Violations: {:?}", violations),
            )
            .into_response());
        }
    }

    Ok(swap_usd_value)
}

async fn build_and_sign_tx(
    state: &AppState,
    agent_id: &str,
    agent_pubkey: Pubkey,
    whirlpool: Pubkey,
    input_mint: Pubkey,
    amount: u64,
    slippage_bps: u16,
) -> Result<VersionedTransaction, Response> {
    let orca_result = state
        .orca_client
        .swap(
            whirlpool,
            amount,
            input_mint,
            SwapType::ExactIn,
            slippage_bps,
            agent_pubkey,
        )
        .await
        .map_err(|e| {
            tracing::error!("Orca swap error for agent {}: {}", agent_id, e);
            ApiResponse::<()>::error(StatusCode::BAD_GATEWAY, e.to_string()).into_response()
        })?;

    let signer = state.agent_signer.load(agent_id).await.map_err(|e| {
        ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
    })?;

    let blockhash = state
        .kora_gateway
        .get_latest_blockhash()
        .await
        .map_err(|e| {
            ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                .into_response()
        })?;

    let kora_pubkey = Pubkey::from_str(&state.config.kora_pubkey).unwrap_or(agent_pubkey);

    let message = Message::new(&orca_result.instructions, Some(&kora_pubkey));
    let mut tx = Transaction::new_unsigned(message);
    tx.message.recent_blockhash = blockhash;

    let message_bytes = tx.message.serialize();
    let keychain_sig = signer.sign_message(&message_bytes).await.map_err(|e| {
        ApiResponse::<()>::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to sign message: {}", e),
        )
        .into_response()
    })?;

    let agent_sig = Signature::from(<[u8; 64]>::from(keychain_sig));

    // Sign for Agent
    for (idx, pk) in tx.message.account_keys.iter().enumerate() {
        if pk == &agent_pubkey && tx.message.is_signer(idx) {
            tx.signatures[idx] = agent_sig;
        }
    }

    // Sign for Orca's ephemeral keypairs
    for kp in orca_result.additional_signers {
        if let Some(idx) = tx
            .message
            .account_keys
            .iter()
            .position(|key| key == &kp.pubkey())
        {
            tx.signatures[idx] = kp.sign_message(&message_bytes);
        }
    }

    Ok(VersionedTransaction::from(tx))
}

async fn write_rejected_audit(state: &AppState, agent_id: &str, violations: Vec<String>) {
    let entry = NewAuditEntry {
        agent_id: agent_id.to_string(),
        instruction_type: InstructionType::TokenSwap.to_string(),
        status: "rejected".to_string(),
        tx_signature: None,
        policy_violations: Some(violations),
        metadata: None,
    };
    let _ = state.audit_store.append(&entry).await;
}

async fn write_confirmed_audit(
    state: &AppState,
    agent_id: &str,
    tx_signature: &str,
    payload: &OrcaSwapRequest,
    slippage_bps: u16,
    usd_volume: f64,
) {
    let entry = NewAuditEntry {
        agent_id: agent_id.to_string(),
        instruction_type: InstructionType::TokenSwap.to_string(),
        status: "confirmed".to_string(),
        tx_signature: Some(tx_signature.to_string()),
        policy_violations: None,
        metadata: Some(serde_json::json!({
            "input_mint": payload.input_mint,
            "amount": payload.amount,
            "slippage_bps": slippage_bps,
            "usd_volume": usd_volume,
        })),
    };
    let _ = state.audit_store.append(&entry).await;
}

fn spawn_orca_confirmation_task(
    state: AppState,
    agent_id: String,
    agent_pubkey: Pubkey,
    tx_sig: Signature,
) {
    let program_id = Pubkey::new_from_array(klave_anchor::ID.to_bytes());
    let (vault_pda, _) =
        Pubkey::find_program_address(&[b"vault", agent_pubkey.as_ref()], &program_id);

    tokio::spawn(async move {
        state.kora_gateway.confirm_transaction(&tx_sig).await;
        let (sol, vault) = match state
            .kora_gateway
            .get_balances(&agent_pubkey, &vault_pda)
            .await
        {
            Ok(balances) => balances,
            Err(e) => {
                tracing::warn!(error = %e, agent_id = %agent_id, "failed to fetch balances for SSE update");
                (0, 0)
            }
        };
        let tokens = state
            .kora_gateway
            .get_token_balances(&agent_pubkey)
            .await
            .unwrap_or_default();

        let _ = state.event_tx.send(ServerEvent::BalanceUpdated {
            agent_id,
            sol_lamports: sol,
            vault_lamports: vault,
            tokens,
        });
    });
}
