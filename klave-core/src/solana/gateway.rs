use base64::{Engine as _, engine::general_purpose};
use reqwest::Client;
use serde_json::json;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::VersionedTransaction;
use std::sync::Arc;

use crate::error::KlaveError;

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
            rpc_client: Arc::new(RpcClient::new(rpc_url)),
            http_client: Client::new(),
        }
    }

    pub async fn get_latest_blockhash(&self) -> Result<solana_sdk::hash::Hash, KlaveError> {
        self.rpc_client
            .get_latest_blockhash()
            .await
            .map_err(|e| KlaveError::Internal(e.to_string()))
    }

    pub async fn get_balances(
        &self,
        agent_pubkey: &solana_sdk::pubkey::Pubkey,
        vault_pda: &solana_sdk::pubkey::Pubkey,
    ) -> Result<(u64, u64), KlaveError> {
        let (native, vault_info) = tokio::try_join!(
            self.rpc_client.get_balance(agent_pubkey),
            self.rpc_client.get_account(vault_pda)
        )
        .map_err(|e| KlaveError::Internal(e.to_string()))?;

        Ok((native, vault_info.lamports))
    }

    pub async fn send_transaction(
        &self,
        tx: &VersionedTransaction,
    ) -> Result<(Signature, bool), KlaveError> {
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
                        if let Ok(bytes) = general_purpose::STANDARD.decode(signed_b64) {
                            if let Ok(fully_signed_tx) =
                                bincode::deserialize::<VersionedTransaction>(&bytes)
                            {
                                match self
                                    .rpc_client
                                    .send_transaction_with_config(
                                        &fully_signed_tx,
                                        solana_client::rpc_config::RpcSendTransactionConfig {
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
        let sig = self
            .rpc_client
            .send_transaction(tx)
            .await
            .map_err(|e| KlaveError::Internal(format!("Fallback RPC Error: {}", e)))?;

        Ok((sig, false))
    }
}
