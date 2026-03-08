use std::str::FromStr;
use uuid::Uuid;

use anchor_lang::{InstructionData, ToAccountMetas};
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use klave_anchor::{
    accounts::{CloseVault, InitializeVault, VaultOperation},
    instruction::{
        CloseVault as CloseInst, Deposit as DepInst, InitializeVault as InitInst,
        Withdraw as WdInst,
    },
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
    signature::Signature,
    transaction::{Transaction, VersionedTransaction},
};
use solana_system_interface::{instruction::transfer, program::ID as SYSTEM_PROGRAM_ID};

use crate::{event::ServerEvent, middleware::AuthContext, response::ApiResponse, state::AppState};

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
    Extension(auth): Extension<AuthContext>,
    Path(agent_id): Path<String>,
    Json(payload): Json<GatewayRequest>,
) -> Response {
    if !auth.is_operator && auth.agent_id.as_deref() != Some(&agent_id) {
        return ApiResponse::<()>::error(StatusCode::FORBIDDEN, "Forbidden".to_string())
            .into_response();
    }

    if Uuid::parse_str(&agent_id).is_err() {
        return ApiResponse::<()>::error(StatusCode::BAD_REQUEST, "Invalid Agent ID format")
            .into_response();
    }

    let agent = match state.agent_repo.find_by_id(&agent_id).await {
        Ok(Some(agent)) => agent,
        Ok(None) => {
            return ApiResponse::<()>::error(StatusCode::NOT_FOUND, "agent not found".to_string())
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
            return ApiResponse::<()>::error(StatusCode::NOT_FOUND, "policy not found".to_string())
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
                "Invalid agent pubkey".to_string(),
            )
            .into_response();
        }
    };

    let program_id = Pubkey::new_from_array(klave_anchor::ID.to_bytes());
    let (vault_pda, _bump) =
        Pubkey::find_program_address(&[b"vault", agent_pubkey.as_ref()], &program_id);

    let (instructions, policy_req) = match build_instructions(
        &payload,
        agent_pubkey,
        vault_pda,
        program_id,
        agent.is_active,
    ) {
        Ok(res) => res,
        Err((status, msg)) => return ApiResponse::<()>::error(status, msg).into_response(),
    };

    let payload_amount = payload.amount.unwrap_or(0);

    let past_spend_usd = state
        .audit_store
        .sum_daily_spend(&agent.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to fetch daily spend");
            ApiResponse::<()>::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to assess daily spend limit".to_string(),
            )
            .into_response()
        })
        .unwrap_or(0.0);
    let tx_usd_value = state.price_feed.lamports_to_usd(payload_amount).await;
    let daily_spend_usd = past_spend_usd + tx_usd_value;

    if let Err(violations) = PolicyEngine::evaluate(&policy, &policy_req, daily_spend_usd) {
        let violation_strings: Vec<String> = violations.iter().map(|val| val.to_string()).collect();
        write_audit_entry(
            state.clone(),
            agent.id.clone(),
            policy_req.instruction_type.clone(),
            "rejected".to_string(),
            "".to_string(),
            payload_amount,
            tx_usd_value,
        )
        .await;
        return ApiResponse::<()>::error(
            StatusCode::FORBIDDEN,
            format!("Policy Violations: {:?}", violation_strings),
        )
        .into_response();
    }

    let (tx_sig, via_kora) =
        match sign_and_broadcast(&state, &agent_id, agent_pubkey, &instructions).await {
            Ok(res) => res,
            Err(resp) => return resp,
        };

    write_audit_entry(
        state.clone(),
        agent.id.clone(),
        policy_req.instruction_type.clone(),
        "confirmed".to_string(),
        tx_sig.to_string(),
        payload_amount,
        tx_usd_value,
    )
    .await;

    tracing::info!(
        agent_id = %agent.id,
        instruction = ?policy_req.instruction_type,
        signature = %tx_sig,
        "Transaction executed successfully"
    );

    let _ = state.event_tx.send(ServerEvent::TransactionExecuted {
        signature: tx_sig.to_string(),
        agent_id: agent_id.clone(),
    });

    spawn_transaction_confirmation_task(state.clone(), agent_id, agent_pubkey, vault_pda, tx_sig);

    ApiResponse::success(
        GatewayResponse {
            signature: tx_sig.to_string(),
            via_kora,
        },
        "transaction sent",
    )
    .into_response()
}

fn build_instructions(
    payload: &GatewayRequest,
    agent_pubkey: Pubkey,
    vault_pda: Pubkey,
    program_id: Pubkey,
    is_active: bool,
) -> Result<(Vec<Instruction>, TransactionRequest), (StatusCode, String)> {
    let instruction_type = match payload.instruction_type.as_str() {
        "sol_transfer" => InstructionType::SolTransfer,
        "initialize_vault" => InstructionType::InitializeVault,
        "deposit_to_vault" => InstructionType::DepositToVault,
        "withdraw_from_vault" => InstructionType::WithdrawFromVault,
        "agent_withdrawal" => InstructionType::AgentWithdrawal,
        "close_vault" => InstructionType::CloseVault,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Unknown instruction type".to_string(),
            ));
        }
    };

    let payload_amount = payload.amount.unwrap_or(0);
    let mut instructions = Vec::new();
    let mut policy_req = TransactionRequest {
        instruction_type: instruction_type.clone(),
        lamports: payload.amount.map(|val| val as i64),
        program_ids: vec![],
        mints: vec![],
        destination: payload.destination.clone(),
        slippage_bps: None,
        is_active,
    };

    let get_anchor_pubkey =
        |key: Pubkey| anchor_lang::prelude::Pubkey::new_from_array(key.to_bytes());

    match instruction_type {
        InstructionType::SolTransfer | InstructionType::AgentWithdrawal => {
            let dest_pubkey = Pubkey::from_str(payload.destination.as_deref().unwrap_or(""))
                .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid destination".to_string()))?;

            instructions.push(transfer(&agent_pubkey, &dest_pubkey, payload_amount));
            policy_req.program_ids.push(SYSTEM_PROGRAM_ID.to_string());
        }

        InstructionType::CloseVault => {
            let accounts = CloseVault {
                vault: get_anchor_pubkey(vault_pda),
                agent: get_anchor_pubkey(agent_pubkey),
                system_program: anchor_lang::solana_program::system_program::ID,
            };

            instructions.push(Instruction {
                program_id,
                accounts: accounts
                    .to_account_metas(Some(false))
                    .into_iter()
                    .map(|account| AccountMeta {
                        pubkey: Pubkey::new_from_array(account.pubkey.to_bytes()),
                        is_signer: account.is_signer,
                        is_writable: account.is_writable,
                    })
                    .collect(),
                data: CloseInst {}.data(),
            });
            policy_req.program_ids.push(program_id.to_string());
        }

        InstructionType::InitializeVault => {
            let accounts = InitializeVault {
                vault: get_anchor_pubkey(vault_pda),
                agent: get_anchor_pubkey(agent_pubkey),
                payer: get_anchor_pubkey(agent_pubkey),
                system_program: get_anchor_pubkey(SYSTEM_PROGRAM_ID),
            };
            instructions.push(Instruction {
                program_id: Pubkey::new_from_array(klave_anchor::ID.to_bytes()),
                accounts: accounts
                    .to_account_metas(Some(false))
                    .into_iter()
                    .map(|account| AccountMeta {
                        pubkey: Pubkey::new_from_array(account.pubkey.to_bytes()),
                        is_signer: account.is_signer,
                        is_writable: account.is_writable,
                    })
                    .collect(),
                data: InitInst {}.data(),
            });
            policy_req.program_ids.push(klave_anchor::ID.to_string());
            policy_req.program_ids.push(SYSTEM_PROGRAM_ID.to_string());
        }

        InstructionType::DepositToVault => {
            let accounts = VaultOperation {
                vault: get_anchor_pubkey(vault_pda),
                agent: get_anchor_pubkey(agent_pubkey),
                system_program: get_anchor_pubkey(SYSTEM_PROGRAM_ID),
            };
            instructions.push(Instruction {
                program_id: Pubkey::new_from_array(klave_anchor::ID.to_bytes()),
                accounts: accounts
                    .to_account_metas(Some(false))
                    .into_iter()
                    .map(|account| AccountMeta {
                        pubkey: Pubkey::new_from_array(account.pubkey.to_bytes()),
                        is_signer: account.is_signer,
                        is_writable: account.is_writable,
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
                vault: get_anchor_pubkey(vault_pda),
                agent: get_anchor_pubkey(agent_pubkey),
                system_program: get_anchor_pubkey(SYSTEM_PROGRAM_ID),
            };
            instructions.push(Instruction {
                program_id: Pubkey::new_from_array(klave_anchor::ID.to_bytes()),
                accounts: accounts
                    .to_account_metas(Some(false))
                    .into_iter()
                    .map(|account| AccountMeta {
                        pubkey: Pubkey::new_from_array(account.pubkey.to_bytes()),
                        is_signer: account.is_signer,
                        is_writable: account.is_writable,
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

    Ok((instructions, policy_req))
}

async fn sign_and_broadcast(
    state: &AppState,
    agent_id: &str,
    agent_pubkey: Pubkey,
    instructions: &[Instruction],
) -> Result<(Signature, bool), Response> {
    let signer = state.agent_signer.load(agent_id).await.map_err(|e| {
        ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
    })?;

    let recent_blockhash = state
        .kora_gateway
        .get_latest_blockhash()
        .await
        .map_err(|e| {
            ApiResponse::<()>::error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                .into_response()
        })?;

    let kora_pubkey = Pubkey::from_str(&state.config.kora_pubkey).unwrap_or(agent_pubkey);
    let mut message = Message::new(instructions, Some(&kora_pubkey));

    message.recent_blockhash = recent_blockhash;

    let mut legacy_tx = Transaction::new_unsigned(message.clone());

    let message_data = message.serialize();
    let keychain_signature = signer.sign_message(&message_data).await.map_err(|e| {
        ApiResponse::<()>::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to sign: {}", e),
        )
        .into_response()
    })?;

    let signature = Signature::try_from(keychain_signature.as_ref()).map_err(|e| {
        ApiResponse::<()>::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Invalid signature: {}", e),
        )
        .into_response()
    })?;

    let mut signers_found = false;
    for (idx, pk) in message.account_keys.iter().enumerate() {
        if pk == &agent_pubkey && message.is_signer(idx) {
            legacy_tx.signatures[idx] = signature;
            signers_found = true;
        }
    }

    if !signers_found {
        return Err(ApiResponse::<()>::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Agent pubkey not found in signers".to_string(),
        )
        .into_response());
    }

    state
        .kora_gateway
        .send_transaction(&VersionedTransaction::from(legacy_tx))
        .await
        .map_err(|e| {
            ApiResponse::<()>::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Transaction failed: {}", e),
            )
            .into_response()
        })
}

async fn write_audit_entry(
    state: AppState,
    agent_id: String,
    instruction_type: InstructionType,
    status: String,
    tx_signature: String,
    payload_amount: u64,
    usd_value: f64,
) {
    let metadata = match instruction_type {
        InstructionType::SolTransfer
        | InstructionType::WithdrawFromVault
        | InstructionType::DepositToVault
        | InstructionType::AgentWithdrawal => {
            Some(serde_json::json!({ "lamports": payload_amount, "usd_value": usd_value }))
        }
        _ => None,
    };

    let entry = NewAuditEntry {
        agent_id,
        instruction_type: instruction_type.to_string(),
        status,
        tx_signature: Some(tx_signature),
        policy_violations: None,
        metadata,
    };
    let _ = state.audit_store.append(&entry).await;
}

fn spawn_transaction_confirmation_task(
    state: AppState,
    agent_id: String,
    agent_pubkey: Pubkey,
    vault_pda: Pubkey,
    tx_sig: Signature,
) {
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
