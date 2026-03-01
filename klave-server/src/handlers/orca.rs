use std::str::FromStr;
use uuid::Uuid;

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use klave_core::{
    policy::engine::{InstructionType, PolicyEngine},
    {agent::model::SwapQuote, audit::store::NewAuditEntry},
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
) -> ApiResponse<OrcaSwapResponse> {
    if !auth.is_operator && auth.agent_id.as_deref() != Some(&id) {
        return ApiResponse::error(StatusCode::FORBIDDEN, "Forbidden");
    }

    let agent_id = match Uuid::from_str(&id) {
        Ok(uuid) => uuid.to_string(),
        Err(_) => return ApiResponse::error(StatusCode::BAD_REQUEST, "invalid agent id format"),
    };

    let agent = match state.agent_repo.find_by_id(&agent_id).await {
        Ok(Some(a)) => a,
        Ok(None) => return ApiResponse::error(StatusCode::NOT_FOUND, "agent not found"),
        Err(e) => return ApiResponse::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let policy = match state.agent_repo.find_policy(&agent_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return ApiResponse::error(StatusCode::NOT_FOUND, "agent policy not found"),
        Err(e) => return ApiResponse::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let slippage_bps = payload.slippage_bps.unwrap_or(policy.slippage_bps as u16);
    let whirlpool = match Pubkey::from_str(&payload.whirlpool) {
        Ok(p) => p,
        Err(_) => return ApiResponse::error(StatusCode::BAD_REQUEST, "invalid whirlpool address"),
    };
    let input_mint = match Pubkey::from_str(&payload.input_mint) {
        Ok(p) => p,
        Err(_) => return ApiResponse::error(StatusCode::BAD_REQUEST, "invalid input mint address"),
    };

    // Static policy check: token allowlist + slippage
    // Determine output mint (the other token in the pair) — we don't know it yet,
    // so we check the input mint here. The output mint is checked implicitly by
    // the pool selection. For the static check, we validate what we know.
    if let Err(violations) = PolicyEngine::check_swap_static(
        &policy,
        &payload.input_mint,
        &payload.input_mint,
        slippage_bps as i32,
    ) {
        let violation_strings: Vec<String> = violations.iter().map(|v| v.to_string()).collect();
        let entry = NewAuditEntry {
            agent_id: agent.id.clone(),
            instruction_type: InstructionType::TokenSwap.to_string(),
            status: "rejected".to_string(),
            tx_signature: None,
            policy_violations: Some(violation_strings.clone()),
            metadata: None,
        };
        let _ = state.audit_store.append(&entry).await;
        return ApiResponse::error(
            StatusCode::FORBIDDEN,
            format!("Policy Violations: {:?}", violation_strings),
        );
    }

    // Swap volume check
    let swap_usd_value = state.price_feed.lamports_to_usd(payload.amount).await;
    let daily_swap_volume = match state.audit_store.sum_swap_volume(&agent_id).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error = %e, "failed to fetch swap volume");
            return ApiResponse::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to assess swap volume limit",
            );
        }
    };
    if let Err(violations) =
        PolicyEngine::check_swap_volume(&policy, swap_usd_value, daily_swap_volume)
    {
        let violation_strings: Vec<String> = violations.iter().map(|v| v.to_string()).collect();
        let entry = NewAuditEntry {
            agent_id: agent.id.clone(),
            instruction_type: InstructionType::TokenSwap.to_string(),
            status: "rejected".to_string(),
            tx_signature: None,
            policy_violations: Some(violation_strings.clone()),
            metadata: None,
        };
        let _ = state.audit_store.append(&entry).await;
        return ApiResponse::error(
            StatusCode::FORBIDDEN,
            format!("Policy Violations: {:?}", violation_strings),
        );
    }

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

    let orca_result = match state
        .orca_client
        .swap(
            whirlpool,
            payload.amount,
            input_mint,
            SwapType::ExactIn,
            slippage_bps,
            agent_pubkey,
        )
        .await
    {
        Ok(res) => res,
        Err(e) => {
            tracing::error!("Orca swap error for agent {}: {}", agent_id, e);
            return ApiResponse::error(StatusCode::BAD_GATEWAY, e.to_string());
        }
    };

    let signer_arc = match state.agent_signer.load(&agent_id).await {
        Ok(s) => s,
        Err(e) => return ApiResponse::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let blockhash = match state.kora_gateway.get_latest_blockhash().await {
        Ok(h) => h,
        Err(e) => return ApiResponse::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let kora_pubkey = match Pubkey::from_str(&state.config.kora_pubkey) {
        Ok(pk) => pk,
        Err(_) => agent_pubkey,
    };

    let message = Message::new(&orca_result.instructions, Some(&kora_pubkey));
    let mut tx = Transaction::new_unsigned(message);
    tx.message.recent_blockhash = blockhash;

    let message_bytes = tx.message.serialize();
    let keychain_sig = match signer_arc.sign_message(&message_bytes).await {
        Ok(s) => s,
        Err(e) => {
            return ApiResponse::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to sign message: {}", e),
            );
        }
    };
    let agent_sig = Signature::from(<[u8; 64]>::from(keychain_sig));

    for (i, pk) in tx.message.account_keys.iter().enumerate() {
        if pk == &agent_pubkey && tx.message.is_signer(i) {
            tx.signatures[i] = agent_sig;
        }
    }

    for kp in orca_result.additional_signers {
        if let Some(idx) = tx
            .message
            .account_keys
            .iter()
            .position(|k| k == &kp.pubkey())
        {
            tx.signatures[idx] = kp.sign_message(&message_bytes);
        }
    }

    let versioned_tx = VersionedTransaction::from(tx);
    let (tx_signature, via_kora) = match state.kora_gateway.send_transaction(&versioned_tx).await {
        Ok(res) => res,
        Err(e) => {
            tracing::error!("Kora transaction error for agent {}: {}", agent.id, e);
            return ApiResponse::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
        }
    };

    tracing::info!(
        agent_id = %agent.id,
        tx_signature = %tx_signature,
        input_mint = %payload.input_mint,
        amount = %payload.amount,
        via_kora = %via_kora,
        "orca swap executed"
    );

    let entry = NewAuditEntry {
        agent_id: agent.id.clone(),
        instruction_type: InstructionType::TokenSwap.to_string(),
        status: "confirmed".to_string(),
        tx_signature: Some(tx_signature.to_string()),
        policy_violations: None,
        metadata: Some(serde_json::json!({
            "input_mint": payload.input_mint,
            "amount": payload.amount,
            "slippage_bps": slippage_bps,
            "usd_volume": swap_usd_value,
        })),
    };
    let _ = state.audit_store.append(&entry).await;

    let tx_sig_str = tx_signature.to_string();
    let _ = state.event_tx.send(ServerEvent::TransactionExecuted {
        signature: tx_sig_str.clone(),
        agent_id: agent.id.clone(),
    });

    {
        let state = state.clone();
        let agent_id = agent.id.clone();
        let agent_pubkey = agent_pubkey;
        let program_id = Pubkey::new_from_array(klave_anchor::ID.to_bytes());
        let (vault_pda, _) =
            Pubkey::find_program_address(&[b"vault", agent_pubkey.as_ref()], &program_id);
        let sig = tx_signature;
        tokio::spawn(async move {
            state.kora_gateway.confirm_transaction(&sig).await;
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
                agent_id,
                sol_lamports: sol,
                vault_lamports: vault,
                tokens,
            });
        });
    }

    ApiResponse::success(
        OrcaSwapResponse {
            tx_signature: tx_sig_str,
            via_kora,
        },
        "orca swap executed successfully",
    )
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
        Ok(p) => p,
        Err(_) => return ApiResponse::error(StatusCode::BAD_REQUEST, "invalid whirlpool address"),
    };
    let input_mint = match Pubkey::from_str(&payload.input_mint) {
        Ok(p) => p,
        Err(_) => return ApiResponse::error(StatusCode::BAD_REQUEST, "invalid input mint address"),
    };

    let agent = match state.agent_repo.find_by_id(&id).await {
        Ok(Some(a)) => a,
        Ok(None) => return ApiResponse::error(StatusCode::NOT_FOUND, "agent not found"),
        Err(e) => return ApiResponse::error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
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
