use std::sync::Arc;

use base64::{Engine as _, engine::general_purpose};
use reqwest::Client;
use serde_json::json;
use solana_account_decoder::UiAccountData;
use solana_client::{
    nonblocking::rpc_client::RpcClient, rpc_config::RpcSendTransactionConfig,
    rpc_request::TokenAccountsFilter,
};
use solana_sdk::{
    hash::Hash, pubkey::Pubkey, signature::Signature, transaction::VersionedTransaction,
};
use solana_transaction_client::versioned::VersionedTransaction as ClientVersionedTransaction;

use crate::{
    agent::model::TokenBalance,
    error::{KlaveError, Result},
};

pub struct KoraGateway {
    kora_rpc_url: String,
    kora_api_key: Option<String>,
    rpc_client: Arc<RpcClient>,
    http_client: Client,
}

impl KoraGateway {
    pub fn new(kora_rpc_url: String, kora_api_key: Option<String>, rpc_url: String) -> Self {
        Self {
            kora_rpc_url,
            kora_api_key,
            rpc_client: Arc::new(RpcClient::new_with_commitment(
                rpc_url,
                solana_commitment_config::CommitmentConfig::confirmed(),
            )),
            http_client: Client::new(),
        }
    }

    pub async fn get_latest_blockhash(&self) -> Result<Hash> {
        self.rpc_client
            .get_latest_blockhash()
            .await
            .map_err(|e| KlaveError::Internal(e.to_string()))
    }

    pub async fn is_reachable(&self) -> bool {
        if self.kora_rpc_url.is_empty() {
            return false;
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(500))
            .build()
            .unwrap_or_else(|_| Client::new());

        match client.get(&self.kora_rpc_url).send().await {
            Ok(resp) => {
                resp.status().is_success()
                    || resp.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED
                    || resp.status() == reqwest::StatusCode::NOT_FOUND
            }
            Err(_) => false,
        }
    }

    pub async fn get_balance(&self, pubkey: &Pubkey) -> Result<u64> {
        self.rpc_client
            .get_balance(pubkey)
            .await
            .map_err(|e| KlaveError::Internal(format!("RPC get_balance failed: {}", e)))
    }

    pub async fn get_balances(
        &self,
        agent_pubkey: &Pubkey,
        vault_pda: &Pubkey,
    ) -> Result<(u64, u64)> {
        let (sol_result, vault_result) =
            tokio::join!(self.get_balance(agent_pubkey), self.get_balance(vault_pda));

        Ok((sol_result?, vault_result?))
    }

    pub async fn get_token_balances(
        &self,
        owner: &Pubkey,
    ) -> Result<Vec<crate::agent::model::TokenBalance>> {
        let spl_token_program = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
            .parse::<Pubkey>()
            .map_err(|e| KlaveError::Internal(format!("Invalid legacy token program ID: {}", e)))?;

        let spl_token_2022_program = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
            .parse::<Pubkey>()
            .map_err(|e| KlaveError::Internal(format!("Invalid Token-2022 program ID: {}", e)))?;

        let (legacy_res, extension_res) = tokio::join!(
            self.rpc_client.get_token_accounts_by_owner(
                owner,
                TokenAccountsFilter::ProgramId(spl_token_program),
            ),
            self.rpc_client.get_token_accounts_by_owner(
                owner,
                TokenAccountsFilter::ProgramId(spl_token_2022_program),
            )
        );

        let mut all_accounts = Vec::new();

        match legacy_res {
            Ok(accounts) => all_accounts.extend(accounts),
            Err(e) => tracing::error!("Failed to fetch legacy token accounts: {}", e),
        }

        match extension_res {
            Ok(accounts) => all_accounts.extend(accounts),
            Err(e) => tracing::error!("Failed to fetch Token-2022 accounts: {}", e),
        }

        let mut balances = Vec::new();
        for keyed_account in all_accounts {
            if let UiAccountData::Json(parsed) = keyed_account.account.data {
                if (parsed.program == "spl-token" || parsed.program == "spl-token-2022")
                    && parsed.parsed.get("type").and_then(|t| t.as_str()) == Some("account")
                {
                    let info = match parsed.parsed.get("info") {
                        Some(info) => info,
                        None => continue,
                    };
                    let mint = info
                        .get("mint")
                        .and_then(|m| m.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let token_amount = match info.get("tokenAmount") {
                        Some(ta) => ta,
                        None => continue,
                    };

                    let amount_str = token_amount
                        .get("amount")
                        .and_then(|a| a.as_str())
                        .unwrap_or("0");
                    let amount = amount_str.parse().unwrap_or(0);
                    let decimals = token_amount
                        .get("decimals")
                        .and_then(|d| d.as_u64())
                        .unwrap_or(0) as u8;
                    let ui_amount = token_amount
                        .get("uiAmount")
                        .and_then(|u| u.as_f64())
                        .unwrap_or(0.0);

                    if amount > 0 {
                        balances.push(TokenBalance {
                            mint,
                            amount,
                            decimals,
                            ui_amount,
                        });
                    }
                }
            }
        }

        tracing::debug!(
            owner = %owner,
            count = balances.len(),
            "fetched token balances"
        );

        Ok(balances)
    }

    pub async fn send_transaction(&self, tx: &VersionedTransaction) -> Result<(Signature, bool)> {
        let bincode_tx = bincode::serialize(tx).map_err(|e| KlaveError::Internal(e.to_string()))?;
        let b64_tx = general_purpose::STANDARD.encode(&bincode_tx);

        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "signTransaction",
            "params": {
                "transaction": b64_tx
            }
        });

        // Try Kora first
        if !self.kora_rpc_url.is_empty() {
            let mut req = self.http_client.post(&self.kora_rpc_url).json(&payload);
            if let Some(key) = &self.kora_api_key {
                req = req.header("x-api-key", key);
            }

            let res = req.send().await;

            match res {
                Ok(resp) if resp.status().is_success() => {
                    let json: serde_json::Value = resp.json().await.unwrap_or_default();
                    if let Some(signed_b64) = json
                        .get("result")
                        .and_then(|r| r.get("signed_transaction"))
                        .and_then(|s| s.as_str())
                    {
                        if let Ok(bytes) = general_purpose::STANDARD.decode(signed_b64)
                            && let Ok(fully_signed_tx) =
                                bincode::deserialize::<ClientVersionedTransaction>(&bytes)
                        {
                            match self
                                .rpc_client
                                .send_transaction_with_config(
                                    &fully_signed_tx,
                                    RpcSendTransactionConfig {
                                        // Kora co-signing adds a round-trip that can make the
                                        // blockhash stale before preflight simulation runs.
                                        // The direct RPC fallback path uses default preflight.
                                        skip_preflight: true,
                                        ..Default::default()
                                    },
                                )
                                .await
                            {
                                Ok(sig) => return Ok((sig, true)),
                                Err(e) => {
                                    return Err(KlaveError::Internal(format!(
                                        "Failed to broadcast Kora-signed tx: {}",
                                        e
                                    )));
                                }
                            }
                        }
                        return Err(KlaveError::Internal(
                            "Failed to decode Kora signed_transaction".into(),
                        ));
                    } else if let Some(err) = json.get("error") {
                        tracing::error!("Kora JSON-RPC error: {:?}", err);
                        return Err(KlaveError::Internal(format!(
                            "Kora JSON-RPC error: {:?}",
                            err
                        )));
                    } else {
                        return Err(KlaveError::Internal(format!(
                            "Kora response missing signed_transaction: {:?}",
                            json
                        )));
                    }
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    tracing::error!("Kora returned non-200: {} - {}", status, body);
                    return Err(KlaveError::Internal(format!(
                        "Kora HTTP Error {} - {}",
                        status, body
                    )));
                }
                Err(e) => {
                    tracing::error!("Kora HTTP request failed: {}", e);
                    // allow fallback if HTTP fails entirely (e.g. Kora offline)
                }
            }
        }

        // Fallback or if Kora URL is empty
        let client_tx: ClientVersionedTransaction =
            bincode::deserialize(&bincode_tx).map_err(|e| KlaveError::Internal(e.to_string()))?;
        let sig = self
            .rpc_client
            .send_transaction(&client_tx)
            .await
            .map_err(|e| KlaveError::Internal(format!("Fallback RPC Error: {}", e)))?;

        Ok((sig, false))
    }

    pub async fn confirm_transaction(&self, signature: &Signature) -> bool {
        for _ in 0..30 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            match self.rpc_client.get_signature_statuses(&[*signature]).await {
                Ok(response) => {
                    if let Some(Some(status)) = response.value.first()
                        && status.err.is_none()
                    {
                        return true;
                    }
                }
                Err(_) => continue,
            }
        }
        false
    }
}
