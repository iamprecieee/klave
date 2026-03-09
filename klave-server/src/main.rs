mod config;
mod event;
mod handlers;
mod middleware;
mod response;
mod router;
mod state;

use std::{io, net::SocketAddr, sync::Arc};

use axum::http::{HeaderName, HeaderValue, Method, header::CONTENT_TYPE};
use solana_client::nonblocking::rpc_client::RpcClient;
use tokio::{net::TcpListener, runtime::Builder};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use klave_core::{
    agent::{repository::AgentRepository, signer::AgentSigner},
    audit::store::AuditStore,
    db::init_pool,
    price::PriceFeed,
    solana::{gateway::KoraGateway, orca::OrcaClient},
};
use tracing_subscriber::EnvFilter;

use crate::config::Config;

fn main() -> anyhow::Result<()> {
    // Custom tokio runtime with larger stack size for worker threads.
    // The Orca SDK's tick array traversal can cause stack overflow with default 2MB stack.
    let runtime = Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(8 * 1024 * 1024) // 8MB stack
        .build()?;

    runtime.block_on(async_main())
}

async fn async_main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .json()
        .with_writer(io::stdout)
        .init();

    let config = Config::from_env();

    let pool = init_pool(&config.database_url).await?;
    info!("database initialized");

    let agent_repo = Arc::new(AgentRepository::new(pool.clone(), config.encryption_key));
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

    let state = state::AppState {
        agent_repo,
        audit_store,
        config: Arc::new(config.clone()),
        agent_signer,
        kora_gateway,
        orca_client,
        price_feed,
        event_tx,
        agent_locks: Arc::new(dashmap::DashMap::new()),
    };

    let cors = if state.config.allowed_origins.contains(&"*".to_string()) {
        CorsLayer::new()
            .allow_methods(Any)
            .allow_headers(Any)
            .allow_origin(Any)
    } else {
        let origins = state
            .config
            .allowed_origins
            .iter()
            .map(|s| s.parse::<HeaderValue>().unwrap())
            .collect::<Vec<_>>();
        CorsLayer::new()
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
            .allow_headers([CONTENT_TYPE, HeaderName::from_static("x-api-key")])
            .allow_origin(origins)
    };
    let app = router::build_router(state).layer(cors);

    let addr: SocketAddr = format!("0.0.0.0:{}", config.port).parse()?;
    let listener = TcpListener::bind(&addr).await?;
    info!(address = %addr, "klave server listening");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
