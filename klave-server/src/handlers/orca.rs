use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use klave_core::audit::store::NewAuditEntry;
use klave_core::policy::engine::InstructionType;
use orca_whirlpools::SwapType;
use solana_keychain::SolanaSigner;
use solana_sdk::{
    message::Message,
    pubkey::Pubkey,
    signature::{Signature, Signer},
    transaction::{Transaction, VersionedTransaction},
};

use crate::{response::ApiResponse, state::AppState};

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

pub async fn execute_swap(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<OrcaSwapRequest>,
) -> ApiResponse<OrcaSwapResponse> {
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

    let orca_result = match state
        .orca_client
        .swap(
            whirlpool,
            payload.amount,
            input_mint,
            SwapType::ExactIn,
            slippage_bps,
            agent.pubkey.parse().unwrap(),
        )
        .await
    {
        Ok(res) => res,
        Err(e) => return ApiResponse::error(StatusCode::BAD_GATEWAY, e.to_string()),
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
        Err(_) => agent.pubkey.parse().unwrap(),
    };

    let message = Message::new(&orca_result.instructions, Some(&kora_pubkey));
    let mut tx = Transaction::new_unsigned(message);
    tx.message.recent_blockhash = blockhash;

    let message_bytes = tx.message.serialize();
    let keychain_sig = signer_arc.sign_message(&message_bytes).await.unwrap();
    let agent_sig = Signature::from(<[u8; 64]>::from(keychain_sig));

    let agent_pubkey = agent.pubkey.parse::<Pubkey>().unwrap();
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
        Err(e) => return ApiResponse::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let entry = NewAuditEntry {
        agent_id: agent.id.clone(),
        instruction_type: InstructionType::TokenSwap.to_string(),
        status: "confirmed".to_string(),
        tx_signature: Some(tx_signature.to_string()),
        policy_violations: None,
        metadata: None,
    };
    let _ = state.audit_store.append(&entry).await;

    ApiResponse::success(
        OrcaSwapResponse {
            tx_signature: tx_signature.to_string(),
            via_kora,
        },
        "orca swap executed successfully",
    )
}
