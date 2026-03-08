use std::str::FromStr;

use axum::extract::State;
use solana_sdk::pubkey::Pubkey;

use crate::{response::ApiResponse, state::AppState};

pub async fn health_check(State(state): State<AppState>) -> ApiResponse<serde_json::Value> {
    let kora_pubkey = Pubkey::from_str(&state.config.kora_pubkey).ok();
    let kora_balance = if let Some(pk) = kora_pubkey {
        match state.kora_gateway.get_balance(&pk).await {
            Ok(balance) => balance,
            Err(e) => {
                tracing::warn!(error = %e, "health check: failed to fetch kora balance");
                0
            }
        }
    } else {
        0
    };

    let data = serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "gateway": {
            "fee_payer": state.config.kora_pubkey,
            "fee_payer_lamports": kora_balance,
        }
    });
    ApiResponse::success(data, "healthy")
}
