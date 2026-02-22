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
    rpc_client: Arc<RpcClient>,
    http_client: Client,
}

impl KoraGateway {
    pub fn new(kora_rpc_url: String, rpc_url: String) -> Self {
        Self {
            kora_rpc_url,
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
            "method": "signAndSendTransaction",
            "params": [
                b64_tx,
                { "encoding": "base64" }
            ]
        });

        // Try Kora first
        if !self.kora_rpc_url.is_empty() {
            if let Ok(resp) = self
                .http_client
                .post(&self.kora_rpc_url)
                .json(&payload)
                .send()
                .await
            {
                if resp.status().is_success() {
                    let json: serde_json::Value = resp.json().await.unwrap_or_default();
                    if let Some(sig_str) = json.get("result").and_then(|r| r.as_str()) {
                        if let Ok(sig) = sig_str.parse::<Signature>() {
                            return Ok((sig, true));
                        }
                    }
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
