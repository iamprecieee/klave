mod config;
mod handlers;
mod middleware;
mod response;
mod router;
mod state;

use std::sync::Arc;

use klave_core::agent::repository::AgentRepository;
use klave_core::agent::signer::AgentSigner;
use klave_core::audit::store::AuditStore;
use klave_core::solana::gateway::KoraGateway;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .json()
        .init();

    let config = config::Config::from_env();

    let pool = klave_core::db::init_pool(&config.database_url).await?;
    info!("database initialized");

    let agent_repo = Arc::new(AgentRepository::new(pool.clone()));
    let audit_store = Arc::new(AuditStore::new(pool));

    let agent_signer = Arc::new(AgentSigner::new(agent_repo.clone()));
    let kora_gateway = Arc::new(KoraGateway::new(
        config.kora_rpc_url.clone(),
        config.solana_rpc_url.clone(),
    ));

    let state = state::AppState {
        agent_repo,
        audit_store,
        config: Arc::new(config.clone()),
        agent_signer,
        kora_gateway,
    };

    let app = router::build_router(state);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!(address = %addr, "klave server listening");

    axum::serve(listener, app).await?;

    Ok(())
}
