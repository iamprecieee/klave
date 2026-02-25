use anchor_lang::{InstructionData, ToAccountMetas};
use axum::{
    Json,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use klave_anchor::accounts::{InitializeVault, VaultOperation};
use klave_anchor::instruction::{
    Deposit as DepInst, InitializeVault as InitInst, Withdraw as WdInst,
};
use klave_core::{
    audit::store::NewAuditEntry,
    policy::engine::{InstructionType, PolicyEngine, TransactionRequest},
};
use serde::{Deserialize, Serialize};
use solana_keychain::SolanaSigner;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    message::Message,
    pubkey::Pubkey,
    transaction::VersionedTransaction,
};
use solana_system_interface::program::ID as SYSTEM_PROGRAM_ID;
use std::str::FromStr;

use crate::{response::ApiResponse, state::AppState};

#[derive(Deserialize)]
pub struct GatewayRequest {
    pub instruction_type: String,
    pub amount: Option<u64>,
    pub destination: Option<String>,
}

#[derive(Serialize)]
pub struct GatewayResponse {
    pub signature: String,
    pub via_kora: bool,
}

pub async fn execute_transaction(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Json(payload): Json<GatewayRequest>,
) -> Response {
    let agent = match state.agent_repo.find_by_id(&agent_id).await {
        Ok(Some(a)) => a,
        Ok(None) => {
            return ApiResponse::<()>::error(
                axum::http::StatusCode::NOT_FOUND,
                "agent not found".to_string(),
            )
            .into_response();
        }
        Err(e) => {
            return ApiResponse::<()>::error(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            )
            .into_response();
        }
    };

    let policy = match state.agent_repo.find_policy(&agent_id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return ApiResponse::<()>::error(
                axum::http::StatusCode::NOT_FOUND,
                "policy not found".to_string(),
            )
            .into_response();
        }
        Err(e) => {
            return ApiResponse::<()>::error(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            )
            .into_response();
        }
    };

    let agent_pubkey = match Pubkey::from_str(&agent.pubkey) {
        Ok(pk) => pk,
        Err(_) => {
            return ApiResponse::<()>::error(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Invalid agent pubkey".to_string(),
            )
            .into_response();
        }
    };

    let program_id = Pubkey::new_from_array(klave_anchor::ID.to_bytes());
    let (vault_pda, _bump) =
        Pubkey::find_program_address(&[b"vault", agent_pubkey.as_ref()], &program_id);

    let mut instructions = Vec::new();

    let instruction_type = match payload.instruction_type.as_str() {
        "sol_transfer" => InstructionType::SolTransfer,
        "initialize_vault" => InstructionType::InitializeVault,
        "deposit_to_vault" => InstructionType::DepositToVault,
        "withdraw_from_vault" => InstructionType::WithdrawFromVault,
        "agent_withdrawal" => InstructionType::AgentWithdrawal,
        _ => {
            return ApiResponse::<()>::error(
                axum::http::StatusCode::BAD_REQUEST,
                "Unknown instruction type".to_string(),
            )
            .into_response();
        }
    };

    let mut policy_req = TransactionRequest {
        instruction_type: instruction_type.clone(),
        lamports: payload.amount.map(|v| v as i64),
        program_ids: vec![],
        mints: vec![],
        destination: payload.destination.clone(),
        slippage_bps: None,
        is_active: agent.is_active,
    };

    let payload_amount = payload.amount.unwrap_or(0);

    let to_anchor =
        |p: solana_sdk::pubkey::Pubkey| anchor_lang::prelude::Pubkey::new_from_array(p.to_bytes());

    match instruction_type {
        InstructionType::SolTransfer | InstructionType::AgentWithdrawal => {
            let dest_pubkey =
                match Pubkey::from_str(payload.destination.as_ref().unwrap_or(&"".to_string())) {
                    Ok(pk) => pk,
                    Err(_) => {
                        return ApiResponse::<()>::error(
                            axum::http::StatusCode::BAD_REQUEST,
                            "Invalid destination".to_string(),
                        )
                        .into_response();
                    }
                };

            instructions.push(solana_system_interface::instruction::transfer(
                &agent_pubkey,
                &dest_pubkey,
                payload_amount,
            ));
            policy_req.program_ids.push(SYSTEM_PROGRAM_ID.to_string());
        }

        InstructionType::InitializeVault => {
            let accounts = InitializeVault {
                vault: to_anchor(vault_pda),
                agent: to_anchor(agent_pubkey),
                payer: to_anchor(agent_pubkey),
                system_program: to_anchor(SYSTEM_PROGRAM_ID),
            };
            instructions.push(Instruction {
                program_id: Pubkey::new_from_array(klave_anchor::ID.to_bytes()),
                accounts: accounts
                    .to_account_metas(Some(false))
                    .into_iter()
                    .map(|a| AccountMeta {
                        pubkey: Pubkey::new_from_array(a.pubkey.to_bytes()),
                        is_signer: a.is_signer,
                        is_writable: a.is_writable,
                    })
                    .collect(),
                data: InitInst {}.data(),
            });
            policy_req.program_ids.push(klave_anchor::ID.to_string());
            policy_req.program_ids.push(SYSTEM_PROGRAM_ID.to_string());
        }

        InstructionType::DepositToVault => {
            let accounts = VaultOperation {
                vault: to_anchor(vault_pda),
                agent: to_anchor(agent_pubkey),
                system_program: to_anchor(SYSTEM_PROGRAM_ID),
            };
            instructions.push(Instruction {
                program_id: Pubkey::new_from_array(klave_anchor::ID.to_bytes()),
                accounts: accounts
                    .to_account_metas(Some(false))
                    .into_iter()
                    .map(|a| AccountMeta {
                        pubkey: Pubkey::new_from_array(a.pubkey.to_bytes()),
                        is_signer: a.is_signer,
                        is_writable: a.is_writable,
                    })
                    .collect(),
                data: DepInst {
                    amount: payload_amount,
                }
                .data(),
            });
            policy_req.program_ids.push(klave_anchor::ID.to_string());
            policy_req.program_ids.push(SYSTEM_PROGRAM_ID.to_string());
        }

        InstructionType::WithdrawFromVault => {
            let accounts = VaultOperation {
                vault: to_anchor(vault_pda),
                agent: to_anchor(agent_pubkey),
                system_program: to_anchor(SYSTEM_PROGRAM_ID),
            };
            instructions.push(Instruction {
                program_id: Pubkey::new_from_array(klave_anchor::ID.to_bytes()),
                accounts: accounts
                    .to_account_metas(Some(false))
                    .into_iter()
                    .map(|a| AccountMeta {
                        pubkey: Pubkey::new_from_array(a.pubkey.to_bytes()),
                        is_signer: a.is_signer,
                        is_writable: a.is_writable,
                    })
                    .collect(),
                data: WdInst {
                    amount: payload_amount,
                }
                .data(),
            });
            policy_req.program_ids.push(klave_anchor::ID.to_string());
        }

        _ => {}
    }

    let daily_spend_usd = match state.audit_store.sum_daily_spend(&agent.id).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "failed to fetch daily spend");
            return ApiResponse::<()>::error(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to assess daily spend limit".to_string(),
            )
            .into_response();
        }
    };

    if let Err(violations) = PolicyEngine::evaluate(&policy, &policy_req, daily_spend_usd) {
        return ApiResponse::<()>::error(
            axum::http::StatusCode::FORBIDDEN,
            format!("Policy Violations: {:?}", violations),
        )
        .into_response();
    }

    let signer = match state.agent_signer.load(&agent_id).await {
        Ok(s) => s,
        Err(e) => {
            return ApiResponse::<()>::error(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            )
            .into_response();
        }
    };

    let recent_blockhash = match state.kora_gateway.get_latest_blockhash().await {
        Ok(h) => h,
        Err(e) => {
            return ApiResponse::<()>::error(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            )
            .into_response();
        }
    };

    let kora_pubkey = match Pubkey::from_str(&state.config.kora_pubkey) {
        Ok(pk) => pk,
        Err(_) => agent_pubkey, // Fallback to agent if Kora pubkey is bad
    };

    let mut message = Message::new(&instructions, Some(&kora_pubkey));
    message.recent_blockhash = recent_blockhash;

    // We must build a legacy transaction to properly allocate signature slots
    // for both the fee payer (Kora) and the agent.
    let mut legacy_tx = solana_sdk::transaction::Transaction::new_unsigned(message.clone());

    let message_data = message.serialize();
    let keychain_signature = match signer.sign_message(&message_data).await {
        Ok(s) => s,
        Err(e) => {
            return ApiResponse::<()>::error(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to sign message: {}", e),
            )
            .into_response();
        }
    };

    let signature =
        solana_sdk::signature::Signature::try_from(keychain_signature.as_ref()).unwrap();

    // Find the agent's position in the signature array and insert the signature.
    // The fee payer (Kora) is always at index 0. If the agent is also a required signer,
    // they will be at index 1 (or 0 if they are paying their own fee).
    let mut signers_found = false;
    for (i, pk) in message.account_keys.iter().enumerate() {
        if pk == &agent_pubkey && message.is_signer(i) {
            legacy_tx.signatures[i] = signature;
            signers_found = true;
        }
    }

    if !signers_found {
        return ApiResponse::<()>::error(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Agent pubkey not found in transaction signers".to_string(),
        )
        .into_response();
    }

    let versioned_tx = VersionedTransaction::from(legacy_tx);

    let (tx_sig, via_kora) = match state.kora_gateway.send_transaction(&versioned_tx).await {
        Ok(res) => res,
        Err(e) => {
            return ApiResponse::<()>::error(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Transaction failed: {}", e),
            )
            .into_response();
        }
    };

    // Write audit log entry
    let metadata = match &policy_req.instruction_type {
        InstructionType::SolTransfer
        | InstructionType::WithdrawFromVault
        | InstructionType::DepositToVault
        | InstructionType::AgentWithdrawal => {
            Some(serde_json::json!({ "lamports": payload_amount }))
        }
        _ => None,
    };

    let entry = NewAuditEntry {
        agent_id: agent.id.clone(),
        instruction_type: policy_req.instruction_type.to_string(),
        status: "confirmed".to_string(),
        tx_signature: Some(tx_sig.to_string()),
        policy_violations: None,
        metadata,
    };
    let _ = state.audit_store.append(&entry).await;

    ApiResponse::success(
        GatewayResponse {
            signature: tx_sig.to_string(),
            via_kora,
        },
        "transaction sent",
    )
    .into_response()
}
