use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use klave_core::audit::store::NewAuditEntry;
use klave_core::policy::engine::{InstructionType, PolicyEngine};
use klave_core::solana::jupiter::{QuoteRequest, SwapRequest as JupSwapRequest};
use solana_keychain::SolanaSigner;

use crate::response::ApiResponse;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct SwapRequestPayload {
    pub input_mint: String,
    pub output_mint: String,
    pub amount: u64,
    pub slippage_bps: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct SwapResponsePayload {
    pub input_amount: u64,
    pub output_amount: u64,
    pub slippage_bps: i32,
    pub tx_signature: String,
    pub via_kora: bool,
}

pub async fn execute_swap(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<SwapRequestPayload>,
) -> ApiResponse<SwapResponsePayload> {
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

    let slippage_bps = payload.slippage_bps.unwrap_or(policy.slippage_bps);

    // 1. Static Policy Check
    if let Err(violations) = PolicyEngine::check_swap_static(
        &policy,
        &payload.input_mint,
        &payload.output_mint,
        slippage_bps,
    ) {
        let entry = NewAuditEntry {
            agent_id: agent.id.clone(),
            instruction_type: InstructionType::TokenSwap.to_string(),
            status: "rejected".to_string(),
            tx_signature: None,
            policy_violations: Some(violations.iter().map(|v| v.to_string()).collect()),
            metadata: None,
        };
        let _ = state.audit_store.append(&entry).await;

        return ApiResponse::error(
            StatusCode::FORBIDDEN,
            format!("policy violation: {:?}", violations),
        );
    }

    // 2. Fetch Quote
    let quote_req = QuoteRequest {
        input_mint: payload.input_mint.clone(),
        output_mint: payload.output_mint.clone(),
        amount: payload.amount,
        slippage_bps: slippage_bps as u16,
        restrict_intermediate_tokens: true,
    };

    let quote = match state.jupiter_client.get_quote(quote_req).await {
        Ok(q) => q,
        Err(e) => return ApiResponse::error(StatusCode::BAD_GATEWAY, e.to_string()),
    };

    // 3. Volume Policy Check
    let current_daily_volume = match state.audit_store.sum_swap_volume(&agent_id).await {
        Ok(v) => v,
        Err(e) => return ApiResponse::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    // Simple stub for USD pricing: assume input amount * 1.0 (devnet stub)
    // Production would require an on-chain oracle or Jupiter price API
    let quote_usd_value = payload.amount as f64 / 1_000_000.0;

    if let Err(violations) =
        PolicyEngine::check_swap_volume(&policy, quote_usd_value, current_daily_volume)
    {
        let entry = NewAuditEntry {
            agent_id: agent.id.clone(),
            instruction_type: InstructionType::TokenSwap.to_string(),
            status: "rejected".to_string(),
            tx_signature: None,
            policy_violations: Some(violations.iter().map(|v| v.to_string()).collect()),
            metadata: None,
        };
        let _ = state.audit_store.append(&entry).await;

        return ApiResponse::error(
            StatusCode::FORBIDDEN,
            format!("policy violation: {:?}", violations),
        );
    }

    // 4. Fetch Swap Transaction
    let swap_req = JupSwapRequest {
        quote_response: quote.clone(),
        user_public_key: agent.pubkey.clone(),
        fee_account: None,
        fee_payer: Some(state.config.kora_pubkey.clone()),
    };

    let mut tx = match state.jupiter_client.get_swap_transaction(swap_req).await {
        Ok(t) => t,
        Err(e) => return ApiResponse::error(StatusCode::BAD_GATEWAY, e.to_string()),
    };

    let signer = match state.agent_signer.load(&agent_id).await {
        Ok(s) => s,
        Err(e) => return ApiResponse::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let message_bytes = tx.message.serialize();
    let signature = match signer.sign_message(&message_bytes).await {
        Ok(sig) => {
            let sig_bytes: [u8; 64] = sig.into();
            solana_sdk::signature::Signature::from(sig_bytes)
        }
        Err(e) => {
            return ApiResponse::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to sign: {:?}", e),
            );
        }
    };

    let required_sigs = tx.message.header().num_required_signatures as usize;
    if tx.signatures.len() != required_sigs {
        tx.signatures
            .resize(required_sigs, solana_sdk::signature::Signature::default());
    }

    let static_keys = tx.message.static_account_keys();
    let signer_pubkey_bytes = signer.pubkey().to_bytes();
    if let Some(idx) = static_keys
        .iter()
        .position(|k| k.to_bytes() == signer_pubkey_bytes)
    {
        if idx < required_sigs {
            tx.signatures[idx] = signature;
        } else {
            return ApiResponse::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "agent key is not a signer",
            );
        }
    } else {
        return ApiResponse::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "agent key not found in static keys",
        );
    }

    let (tx_signature, via_kora) = match state.kora_gateway.send_transaction(&tx).await {
        Ok(res) => res,
        Err(e) => return ApiResponse::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    // Write audit log entry (stubbed success)
    let metadata = serde_json::json!({
        "usd_volume": quote_usd_value,
        "input_amount": payload.amount,
        "output_amount": quote.out_amount,
        "price_impact_pct": quote.price_impact_pct,
    });

    let entry = NewAuditEntry {
        agent_id: agent.id.clone(),
        instruction_type: InstructionType::TokenSwap.to_string(),
        status: "confirmed".to_string(),
        tx_signature: Some(tx_signature.to_string()),
        policy_violations: None,
        metadata: Some(metadata),
    };
    let _ = state.audit_store.append(&entry).await;

    ApiResponse::success(
        SwapResponsePayload {
            input_amount: payload.amount,
            output_amount: quote.out_amount.parse().unwrap_or(0),
            slippage_bps,
            tx_signature: tx_signature.to_string(),
            via_kora,
        },
        "swap executed successfully",
    )
}
