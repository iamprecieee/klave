use std::sync::Arc;

use dashmap::DashMap;
use klave_core::{
    agent::{repository::AgentRepository, signer::AgentSigner},
    audit::store::AuditStore,
    price::PriceFeed,
    solana::{gateway::KoraGateway, orca::OrcaClient},
};
use tokio::sync::broadcast;

use crate::{config::Config, event::ServerEvent};

#[derive(Clone)]
pub struct AppState {
    pub agent_repo: Arc<AgentRepository>,
    pub audit_store: Arc<AuditStore>,
    pub config: Arc<Config>,
    pub agent_signer: Arc<AgentSigner>,
    pub kora_gateway: Arc<KoraGateway>,
    pub orca_client: Arc<OrcaClient>,
    pub price_feed: Arc<PriceFeed>,
    pub event_tx: broadcast::Sender<ServerEvent>,
    pub agent_locks: Arc<DashMap<String, Arc<tokio::sync::Mutex<()>>>>,
}
