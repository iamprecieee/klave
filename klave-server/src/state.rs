use std::sync::Arc;

use klave_core::agent::repository::AgentRepository;
use klave_core::agent::signer::AgentSigner;
use klave_core::audit::store::AuditStore;
use klave_core::solana::gateway::KoraGateway;
use klave_core::solana::jupiter::JupiterClient;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub agent_repo: Arc<AgentRepository>,
    pub audit_store: Arc<AuditStore>,
    pub config: Arc<Config>,
    pub agent_signer: Arc<AgentSigner>,
    pub kora_gateway: Arc<KoraGateway>,
    pub jupiter_client: Arc<JupiterClient>,
}
