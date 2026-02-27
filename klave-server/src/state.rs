use std::sync::Arc;

use klave_core::{
    agent::{repository::AgentRepository, signer::AgentSigner},
    audit::store::AuditStore,
    price::PriceFeed,
    solana::{gateway::KoraGateway, orca::OrcaClient},
};

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub agent_repo: Arc<AgentRepository>,
    pub audit_store: Arc<AuditStore>,
    pub config: Arc<Config>,
    pub agent_signer: Arc<AgentSigner>,
    pub kora_gateway: Arc<KoraGateway>,
    pub orca_client: Arc<OrcaClient>,
    pub price_feed: Arc<PriceFeed>,
}
