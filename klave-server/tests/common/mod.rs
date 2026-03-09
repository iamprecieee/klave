use klave_core::{
    agent::{model::AgentPolicyInput, repository::AgentRepository, signer::AgentSigner},
    audit::store::AuditStore,
    db::init_pool,
    price::PriceFeed,
    solana::{gateway::KoraGateway, orca::OrcaClient},
};
use klave_server::{config::Config, router::build_router, state::AppState};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::{env, net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;

pub async fn spawn_test_app() -> String {
    dotenvy::dotenv().ok();
    // For deterministic operator key
    unsafe {
        env::set_var("KLAVE_OPERATOR_API_KEY", "test-operator-key");
    }

    let config = Config {
        port: 0,
        kora_rpc_url: "http://localhost:1337".to_string(),
        kora_api_key: Some("test".to_string()),
        solana_rpc_url: "https://api.devnet.solana.com".to_string(),
        jupiter_api_key: env::var("JUPITER_API_KEY").ok(),
        operator_api_key: "test-operator-key".to_string(),
        kora_pubkey: "DX1HENroMLHzRJjFbHGGcEbAhrNva7mm6zVjxFzEyVe".to_string(),
        encryption_key: [0u8; 32],
        database_url: "sqlite::memory:".to_string(),
        allowed_origins: vec!["*".to_string()],
    };

    let pool = init_pool(&config.database_url).await.unwrap();

    let agent_repo = Arc::new(AgentRepository::new(
        pool.clone(),
        config.encryption_key.clone(),
    ));
    let audit_store = Arc::new(AuditStore::new(pool));

    let agent_signer = Arc::new(AgentSigner::new(agent_repo.clone()));
    let kora_gateway = Arc::new(KoraGateway::new(
        config.kora_rpc_url.clone(),
        config.kora_api_key.clone(),
        config.solana_rpc_url.clone(),
    ));

    let rpc_client = Arc::new(RpcClient::new(config.solana_rpc_url.clone()));
    let orca_client = Arc::new(OrcaClient::new(rpc_client));

    let price_feed = Arc::new(PriceFeed::new(config.jupiter_api_key.clone()));

    let (event_tx, _) = tokio::sync::broadcast::channel(1024);

    let state = AppState {
        agent_repo,
        audit_store,
        config: Arc::new(config),
        agent_signer,
        kora_gateway,
        orca_client,
        price_feed,
        event_tx,
        agent_locks: Arc::new(dashmap::DashMap::new()),
    };

    let router = build_router(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(
            listener,
            router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });

    format!("http://{}", addr)
}

pub async fn setup_test_agent(base_url: &str, client: &reqwest::Client) -> String {
    let response = client
        .post(&format!("{}/api/v1/agents", base_url))
        .json(&serde_json::json!({
            "label": "test-agent",
            "policy": AgentPolicyInput {
                allowed_programs: Some(vec![]),
                max_lamports_per_tx: Some(0),
                token_allowlist: Some(vec![]),
                daily_spend_limit_usd: Some(0.0),
                daily_swap_volume_usd: Some(0.0),
                slippage_bps: Some(0),
                withdrawal_destinations: Some(vec![]),
            }
        }))
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status().as_u16(), 201);
    let body: serde_json::Value = response.json().await.unwrap();
    body["data"]["id"].as_str().unwrap().to_string()
}
