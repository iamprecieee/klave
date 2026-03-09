mod common;

use klave_core::price::PriceFeed;

use crate::common::{setup_test_agent, spawn_test_app};

#[tokio::test]
async fn health_check_works() {
    let base_url = spawn_test_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/health", base_url))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["data"]["status"], "ok");
}

#[tokio::test]
async fn test_create_and_list_agents() {
    let base_url = spawn_test_app().await;
    let client = reqwest::Client::new();

    let agent_id = setup_test_agent(&base_url, &client).await;
    assert!(!agent_id.is_empty());

    let reject_resp = client
        .get(&format!("{}/api/v1/agents", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(reject_resp.status().as_u16(), 401);

    let operator_resp = client
        .get(&format!("{}/api/v1/agents", base_url))
        .header("x-api-key", "test-operator-key")
        .send()
        .await
        .unwrap();

    assert_eq!(operator_resp.status().as_u16(), 200);
    let agents: serde_json::Value = operator_resp.json().await.unwrap();

    let agents_array = agents["data"].as_array().unwrap();
    assert_eq!(agents_array.len(), 1);
    assert_eq!(agents_array[0]["id"], agent_id);
}

#[tokio::test]
async fn test_missing_agent_lookup() {
    let base_url = spawn_test_app().await;
    let client = reqwest::Client::new();

    let bad_agent_resp = client
        .get(&format!("{}/api/v1/agents/bad-uuid/balance", base_url))
        .header("x-api-key", "test-operator-key")
        .send()
        .await
        .unwrap();

    assert_eq!(bad_agent_resp.status().as_u16(), 400);
}

#[tokio::test]
async fn test_agent_deactivation() {
    let base_url = spawn_test_app().await;
    let client = reqwest::Client::new();

    let agent_id = setup_test_agent(&base_url, &client).await;

    let deactivate_resp = client
        .delete(&format!("{}/api/v1/agents/{}", base_url, agent_id))
        .header("x-api-key", "test-operator-key")
        .send()
        .await
        .unwrap();
    assert_eq!(deactivate_resp.status().as_u16(), 204);
}

#[tokio::test]
async fn test_agent_policy_update() {
    let base_url = spawn_test_app().await;
    let client = reqwest::Client::new();

    let agent_id = setup_test_agent(&base_url, &client).await;

    let update_policy_resp = client
        .put(&format!("{}/api/v1/agents/{}/policy", base_url, agent_id))
        .header("x-api-key", "test-operator-key")
        .json(&serde_json::json!({
            "allowed_programs": vec!["11111111111111111111111111111111"],
            "max_lamports_per_tx": 500000,
            "token_allowlist": Vec::<String>::new(),
            "daily_spend_limit_usd": 1500.0,
            "daily_swap_volume_usd": 500.0,
            "slippage_bps": 300,
            "withdrawal_destinations": Vec::<String>::new()
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(update_policy_resp.status().as_u16(), 200);
}

#[tokio::test]
async fn test_agent_tracking_endpoints() {
    let base_url = spawn_test_app().await;
    let client = reqwest::Client::new();

    let agent_id = setup_test_agent(&base_url, &client).await;

    let history_resp = client
        .get(&format!("{}/api/v1/agents/{}/history", base_url, agent_id))
        .header("x-api-key", "test-operator-key")
        .send()
        .await
        .unwrap();
    assert_eq!(history_resp.status().as_u16(), 200);

    let tokens_resp = client
        .get(&format!("{}/api/v1/agents/{}/tokens", base_url, agent_id))
        .header("x-api-key", "test-operator-key")
        .send()
        .await
        .unwrap();
    assert_eq!(tokens_resp.status().as_u16(), 200);
}

#[tokio::test]
async fn test_transaction_policy_enforcement() {
    let base_url = spawn_test_app().await;
    let client = reqwest::Client::new();

    let agent_id = setup_test_agent(&base_url, &client).await;

    let tx_resp = client
        .post(&format!(
            "{}/api/v1/agents/{}/transactions",
            base_url, agent_id
        ))
        .header("x-api-key", "test-operator-key")
        .json(&serde_json::json!({
            "instruction_type": "sol_transfer",
            "amount": 1000000,
            "destination": "DX1HENroMLHzRJjFbHGGcEbAhrNva7mm6zVjxFzEyVe"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(tx_resp.status().as_u16(), 403);
}

#[tokio::test]
async fn test_jupiter_price_feed() {
    // This test requires a JUPITER_API_KEY to be set in .env
    dotenvy::dotenv().ok();
    let api_key = std::env::var("JUPITER_API_KEY").ok();
    assert!(
        api_key.is_some(),
        "JUPITER_API_KEY must be set in .env for this test"
    );

    let feed = PriceFeed::new(api_key);
    let usd_per_sol = feed.lamports_to_usd(1_000_000_000).await;

    println!("Current SOL price from Jupiter: ${}", usd_per_sol);
    assert!(
        usd_per_sol > 0.0,
        "SOL price should be fetched and non-zero"
    );
}

#[tokio::test]
async fn test_swap_policy_enforcement() {
    let base_url = spawn_test_app().await;
    let client = reqwest::Client::new();

    let sol_mint = "So11111111111111111111111111111111111111112";
    let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    let whirlpool = "GJ4soixQZbZzHgZvuK522QLqj87ZcYn5zUE2PKPesKWs";

    let response = client
        .post(&format!("{}/api/v1/agents", base_url))
        .json(&serde_json::json!({
            "label": "policy-test-agent",
            "policy": {
                "allowed_programs": vec!["whir7mC6WjX7qZ9S2ghfS2xny12y1Pq51H6Bnn1FfV2"], // Orca Program
                "max_lamports_per_tx": 10_000_000_000_i64,
                "token_allowlist": vec![sol_mint, usdc_mint],
                "daily_spend_limit_usd": 1000.0,
                "daily_swap_volume_usd": 0.01, // $0.01 limit
                "slippage_bps": 100,
                "withdrawal_destinations": Vec::<String>::new()
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 201);
    let body: serde_json::Value = response.json().await.unwrap();
    let agent_id = body["data"]["id"].as_str().unwrap().to_string();

    let swap_resp = client
        .post(&format!(
            "{}/api/v1/agents/{}/orca/swap",
            base_url, agent_id
        ))
        .header("x-api-key", "test-operator-key")
        .json(&serde_json::json!({
            "whirlpool": whirlpool,
            "input_mint": sol_mint,
            "output_mint": usdc_mint,
            "amount": 100_000_000, // 0.1 SOL
            "slippage_bps": 50
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(swap_resp.status().as_u16(), 403);
    let error_body: serde_json::Value = swap_resp.json().await.unwrap();
    assert!(
        error_body["message"]
            .as_str()
            .unwrap()
            .contains("daily swap volume")
    );
}
