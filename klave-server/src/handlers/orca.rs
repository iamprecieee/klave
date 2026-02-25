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
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::VersionedTransaction;

use crate::response::ApiResponse;
use crate::state::AppState;

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

    let message = solana_sdk::message::Message::new(&orca_result.instructions, Some(&kora_pubkey));
    let mut tx = solana_sdk::transaction::Transaction::new_unsigned(message);
    tx.message.recent_blockhash = blockhash;

    let message_bytes = tx.message.serialize();
    let keychain_sig = signer_arc.sign_message(&message_bytes).await.unwrap();
    let agent_sig = solana_sdk::signature::Signature::from(<[u8; 64]>::from(keychain_sig));

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

#[derive(Debug, Deserialize)]
pub struct OpenPositionRequest {
    pub whirlpool: String,
    pub token_max_a: u64,
    pub token_max_b: u64,
    pub slippage_bps: Option<u16>,
}

pub async fn open_position(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<OpenPositionRequest>,
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

    let result = match state
        .orca_client
        .open_splash_position(
            whirlpool,
            payload.token_max_a,
            payload.token_max_b,
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

    let message = solana_sdk::message::Message::new(&result.instructions, Some(&kora_pubkey));
    let mut tx = solana_sdk::transaction::Transaction::new_unsigned(message);
    tx.message.recent_blockhash = blockhash;

    let message_bytes = tx.message.serialize();
    let keychain_sig = signer_arc.sign_message(&message_bytes).await.unwrap();
    let agent_sig = solana_sdk::signature::Signature::from(<[u8; 64]>::from(keychain_sig));

    let agent_pubkey = agent.pubkey.parse::<Pubkey>().unwrap();
    for (i, pk) in tx.message.account_keys.iter().enumerate() {
        if pk == &agent_pubkey && tx.message.is_signer(i) {
            tx.signatures[i] = agent_sig;
        }
    }

    for kp in result.additional_signers {
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
        instruction_type: InstructionType::OrcaLiquidityProvision.to_string(),
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
        "position opened successfully",
    )
}

#[derive(Debug, Deserialize)]
pub struct IncreaseLiquidityRequest {
    pub position: String,
    pub amount_a: u64,
    pub amount_b: u64,
    pub slippage_bps: Option<u16>,
}

pub async fn increase_liquidity(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<IncreaseLiquidityRequest>,
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

    let position = match Pubkey::from_str(&payload.position) {
        Ok(p) => p,
        Err(_) => return ApiResponse::error(StatusCode::BAD_REQUEST, "invalid position address"),
    };
    let slippage_bps = payload.slippage_bps.unwrap_or(policy.slippage_bps as u16);

    let result = match state
        .orca_client
        .increase_liquidity(
            position,
            payload.amount_a,
            payload.amount_b,
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

    let message = solana_sdk::message::Message::new(&result.instructions, Some(&kora_pubkey));
    let mut tx = solana_sdk::transaction::Transaction::new_unsigned(message);
    tx.message.recent_blockhash = blockhash;

    let message_bytes = tx.message.serialize();
    let keychain_sig = signer_arc.sign_message(&message_bytes).await.unwrap();
    let agent_sig = solana_sdk::signature::Signature::from(<[u8; 64]>::from(keychain_sig));

    let agent_pubkey = agent.pubkey.parse::<Pubkey>().unwrap();
    for (i, pk) in tx.message.account_keys.iter().enumerate() {
        if pk == &agent_pubkey && tx.message.is_signer(i) {
            tx.signatures[i] = agent_sig;
        }
    }

    for kp in result.additional_signers {
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
        instruction_type: InstructionType::OrcaLiquidityProvision.to_string(),
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
        "liquidity increased successfully",
    )
}

#[derive(Debug, Deserialize)]
pub struct DecreaseLiquidityRequest {
    pub position: String,
    pub liquidity: u128,
    pub slippage_bps: Option<u16>,
}

pub async fn decrease_liquidity(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<DecreaseLiquidityRequest>,
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

    let position = match Pubkey::from_str(&payload.position) {
        Ok(p) => p,
        Err(_) => return ApiResponse::error(StatusCode::BAD_REQUEST, "invalid position address"),
    };
    let slippage_bps = payload.slippage_bps.unwrap_or(policy.slippage_bps as u16);

    let result = match state
        .orca_client
        .decrease_liquidity(
            position,
            payload.liquidity,
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

    let message = solana_sdk::message::Message::new(&result.instructions, Some(&kora_pubkey));
    let mut tx = solana_sdk::transaction::Transaction::new_unsigned(message);
    tx.message.recent_blockhash = blockhash;

    let message_bytes = tx.message.serialize();
    let keychain_sig = signer_arc.sign_message(&message_bytes).await.unwrap();
    let agent_sig = solana_sdk::signature::Signature::from(<[u8; 64]>::from(keychain_sig));

    let agent_pubkey = agent.pubkey.parse::<Pubkey>().unwrap();
    for (i, pk) in tx.message.account_keys.iter().enumerate() {
        if pk == &agent_pubkey && tx.message.is_signer(i) {
            tx.signatures[i] = agent_sig;
        }
    }

    for kp in result.additional_signers {
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
        instruction_type: InstructionType::OrcaLiquidityProvision.to_string(),
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
        "liquidity decreased successfully",
    )
}

#[derive(Debug, Deserialize)]
pub struct HarvestRequest {
    pub position: String,
}

pub async fn harvest(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<HarvestRequest>,
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

    let position = match Pubkey::from_str(&payload.position) {
        Ok(p) => p,
        Err(_) => return ApiResponse::error(StatusCode::BAD_REQUEST, "invalid position address"),
    };

    let result = match state
        .orca_client
        .harvest_position(position, agent.pubkey.parse().unwrap())
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

    let message = solana_sdk::message::Message::new(&result.instructions, Some(&kora_pubkey));
    let mut tx = solana_sdk::transaction::Transaction::new_unsigned(message);
    tx.message.recent_blockhash = blockhash;

    let message_bytes = tx.message.serialize();
    let keychain_sig = signer_arc.sign_message(&message_bytes).await.unwrap();
    let agent_sig = solana_sdk::signature::Signature::from(<[u8; 64]>::from(keychain_sig));

    let agent_pubkey = agent.pubkey.parse::<Pubkey>().unwrap();
    for (i, pk) in tx.message.account_keys.iter().enumerate() {
        if pk == &agent_pubkey && tx.message.is_signer(i) {
            tx.signatures[i] = agent_sig;
        }
    }

    for kp in result.additional_signers {
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
        instruction_type: InstructionType::OrcaLiquidityProvision.to_string(),
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
        "harvested rewards successfully",
    )
}

#[derive(Debug, Deserialize)]
pub struct ClosePositionRequest {
    pub slippage_bps: Option<u16>,
}

pub async fn close_position(
    State(state): State<AppState>,
    Path((id, position_str)): Path<(String, String)>,
    Json(payload): Json<ClosePositionRequest>,
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

    let position = match Pubkey::from_str(&position_str) {
        Ok(p) => p,
        Err(_) => return ApiResponse::error(StatusCode::BAD_REQUEST, "invalid position address"),
    };
    let slippage_bps = payload.slippage_bps.unwrap_or(policy.slippage_bps as u16);

    let result = match state
        .orca_client
        .close_position(position, slippage_bps, agent.pubkey.parse().unwrap())
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

    let message = solana_sdk::message::Message::new(&result.instructions, Some(&kora_pubkey));
    let mut tx = solana_sdk::transaction::Transaction::new_unsigned(message);
    tx.message.recent_blockhash = blockhash;

    let message_bytes = tx.message.serialize();
    let keychain_sig = signer_arc.sign_message(&message_bytes).await.unwrap();
    let agent_sig = solana_sdk::signature::Signature::from(<[u8; 64]>::from(keychain_sig));

    let agent_pubkey = agent.pubkey.parse::<Pubkey>().unwrap();
    for (i, pk) in tx.message.account_keys.iter().enumerate() {
        if pk == &agent_pubkey && tx.message.is_signer(i) {
            tx.signatures[i] = agent_sig;
        }
    }

    for kp in result.additional_signers {
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
        instruction_type: InstructionType::OrcaLiquidityProvision.to_string(),
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
        "position closed successfully",
    )
}
