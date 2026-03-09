use std::str::FromStr;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use solana_sdk::pubkey::Pubkey;

use crate::{response::ApiResponse, state::AppState};

pub async fn health_check(State(state): State<AppState>) -> Response {
    let mut db_ok = true;
    let mut rpc_ok = true;

    if let Err(e) = state.agent_repo.ping().await {
        tracing::error!("health check: database ping failed: {}", e);
        db_ok = false;
    }

    let kora_pubkey = Pubkey::from_str(&state.config.kora_pubkey).ok();
    let kora_balance = if let Some(pk) = kora_pubkey {
        match state.kora_gateway.get_balance(&pk).await {
            Ok(balance) => balance,
            Err(e) => {
                tracing::error!("health check: solana rpc ping failed: {}", e);
                rpc_ok = false;
                0
            }
        }
    } else {
        rpc_ok = false;
        0
    };

    let is_healthy = db_ok && rpc_ok;

    let status_str = if is_healthy { "ok" } else { "error" };
    let status_code = if is_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let data = serde_json::json!({
        "status": status_str,
        "version": env!("CARGO_PKG_VERSION"),
        "components": {
            "database": if db_ok { "ok" } else { "error" },
            "solana_rpc": if rpc_ok { "ok" } else { "error" }
        },
        "gateway": {
            "fee_payer": state.config.kora_pubkey,
            "fee_payer_lamports": kora_balance,
        }
    });

    ApiResponse::success(data, if is_healthy { "healthy" } else { "unhealthy" })
        .with_status(status_code)
        .into_response()
}
