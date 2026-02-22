use anchor_lang::{InstructionData, ToAccountMetas};
use axum::{
    Json,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use klave_anchor::accounts::{Deposit, InitializeVault, Withdraw};
use klave_anchor::instruction::{
    Deposit as DepInst, InitializeVault as InitInst, Withdraw as WdInst,
};
use klave_core::policy::engine::{InstructionType, PolicyEngine, TransactionRequest};
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

    let to_anchor =
        |p: solana_sdk::pubkey::Pubkey| anchor_lang::prelude::Pubkey::new_from_array(p.to_bytes());

    match instruction_type {
        InstructionType::SolTransfer | InstructionType::AgentWithdrawal => {
            let amount = payload.amount.unwrap_or(0);
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
                amount,
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
            let amount = payload.amount.unwrap_or(0);
            let accounts = Deposit {
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
                data: DepInst { amount }.data(),
            });
            policy_req.program_ids.push(klave_anchor::ID.to_string());
            policy_req.program_ids.push(SYSTEM_PROGRAM_ID.to_string());
        }
        InstructionType::WithdrawFromVault => {
            let amount = payload.amount.unwrap_or(0);
            let accounts = Withdraw {
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
                data: WdInst { amount }.data(),
            });
            policy_req.program_ids.push(klave_anchor::ID.to_string());
        }
        _ => {}
    }

    // TODO: fetch real daily spend from audit store
    let daily_spend_usd = 0.0;

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

    let mut message = Message::new(&instructions, Some(&agent_pubkey));
    message.recent_blockhash = recent_blockhash;

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

    let versioned_tx = VersionedTransaction {
        signatures: vec![signature],
        message: solana_sdk::message::VersionedMessage::Legacy(message),
    };

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

    ApiResponse::success(
        serde_json::to_value(GatewayResponse {
            signature: tx_sig.to_string(),
            via_kora,
        })
        .unwrap(),
        "transaction sent",
    )
    .into_response()
}
