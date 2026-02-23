use base64::{Engine as _, engine::general_purpose};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use solana_sdk::transaction::VersionedTransaction;

use crate::error::KlaveError;

pub struct JupiterClient {
    client: Client,
    base_url: String,
    api_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteRequest {
    pub input_mint: String,
    pub output_mint: String,
    pub amount: u64,
    pub slippage_bps: u16,
    pub restrict_intermediate_tokens: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QuoteResponse {
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
    pub other_amount_threshold: String,
    pub swap_mode: String,
    pub slippage_bps: u16,
    pub price_impact_pct: String,
    pub route_plan: Vec<serde_json::Value>,
    #[serde(flatten)]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapRequest {
    pub quote_response: QuoteResponse,
    pub user_public_key: String,
    pub fee_account: Option<String>,
    pub fee_payer: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapResponse {
    pub swap_transaction: String,
    pub last_valid_block_height: u64,
}

impl JupiterClient {
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url,
            api_key,
        }
    }

    pub async fn get_quote(&self, req: QuoteRequest) -> Result<QuoteResponse, KlaveError> {
        let url = format!("{}/quote", self.base_url);

        let mut query_params = vec![
            ("inputMint", req.input_mint),
            ("outputMint", req.output_mint),
            ("amount", req.amount.to_string()),
            ("slippageBps", req.slippage_bps.to_string()),
        ];

        if req.restrict_intermediate_tokens {
            query_params.push(("restrictIntermediateTokens", "true".to_string()));
        }

        let mut request = self.client.get(&url).query(&query_params);

        if let Some(ref key) = self.api_key {
            request = request.header("x-api-key", key);
        }

        let resp = request
            .send()
            .await
            .map_err(|e| KlaveError::Internal(format!("Jupiter quote request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(KlaveError::Internal(format!(
                "Jupiter returned {}: {}",
                status, body
            )));
        }

        let quote: QuoteResponse = resp
            .json()
            .await
            .map_err(|e| KlaveError::Internal(format!("Failed to parse quote: {}", e)))?;
        Ok(quote)
    }

    pub async fn get_swap_transaction(
        &self,
        req: SwapRequest,
    ) -> Result<VersionedTransaction, KlaveError> {
        let url = format!("{}/swap", self.base_url);
        let mut request = self.client.post(&url).json(&req);

        if let Some(ref key) = self.api_key {
            request = request.header("x-api-key", key);
        }

        let resp = request
            .send()
            .await
            .map_err(|e| KlaveError::Internal(format!("Jupiter swap request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(KlaveError::Internal(format!(
                "Jupiter returned {}: {}",
                status, body
            )));
        }

        let swap_resp: SwapResponse = resp
            .json()
            .await
            .map_err(|e| KlaveError::Internal(format!("Failed to parse swap response: {}", e)))?;

        // Decode base64 transaction
        let tx_bytes = general_purpose::STANDARD
            .decode(&swap_resp.swap_transaction)
            .map_err(|e| {
                KlaveError::Internal(format!("Invalid base64 in swap_transaction: {}", e))
            })?;

        // Deserialize VersionedTransaction
        let tx: VersionedTransaction = bincode::deserialize(&tx_bytes).map_err(|e| {
            KlaveError::Internal(format!("Failed to deserialize transaction: {}", e))
        })?;

        Ok(tx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::message::{VersionedMessage, v0};
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn dummy_transaction() -> String {
        let msg = VersionedMessage::V0(v0::Message::default());
        let tx = VersionedTransaction {
            signatures: vec![],
            message: msg,
        };
        let bytes = bincode::serialize(&tx).unwrap();
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    }

    #[tokio::test]
    async fn test_jupiter_client_quote_and_swap() {
        let mock_server = MockServer::start().await;
        let client = JupiterClient::new(mock_server.uri(), None);

        let quote_resp = QuoteResponse {
            input_mint: "So1111".to_string(),
            output_mint: "EPjFWd".to_string(),
            in_amount: "1000000".to_string(),
            out_amount: "2000000".to_string(),
            other_amount_threshold: "1900000".to_string(),
            swap_mode: "ExactIn".to_string(),
            slippage_bps: 50,
            price_impact_pct: "0.1".to_string(),
            route_plan: vec![],
            extra_fields: serde_json::Map::new(),
        };

        Mock::given(method("GET"))
            .and(path("/quote"))
            .and(query_param("inputMint", "So1111"))
            .and(query_param("outputMint", "EPjFWd"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&quote_resp))
            .mount(&mock_server)
            .await;

        let req = QuoteRequest {
            input_mint: "So1111".to_string(),
            output_mint: "EPjFWd".to_string(),
            amount: 1000000,
            slippage_bps: 50,
            restrict_intermediate_tokens: true,
        };

        let result = client.get_quote(req).await.unwrap();
        assert_eq!(result.in_amount, "1000000");
        assert_eq!(result.out_amount, "2000000");

        let swap_resp = SwapResponse {
            swap_transaction: dummy_transaction(),
            last_valid_block_height: 1000,
        };

        Mock::given(method("POST"))
            .and(path("/swap"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&swap_resp))
            .mount(&mock_server)
            .await;

        let swap_req = SwapRequest {
            quote_response: quote_resp,
            user_public_key: "Agent111111111111111111111111111111111111111".to_string(),
            fee_account: None,
            fee_payer: None,
        };

        let tx = client.get_swap_transaction(swap_req).await.unwrap();
        assert!(tx.signatures.is_empty());
    }
}
