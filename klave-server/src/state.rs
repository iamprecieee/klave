use std::sync::Arc;

use klave_core::agent::repository::AgentRepository;
use klave_core::agent::signer::AgentSigner;
use klave_core::audit::store::AuditStore;
use klave_core::solana::gateway::KoraGateway;
use klave_core::solana::orca::OrcaClient;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub agent_repo: Arc<AgentRepository>,
    pub audit_store: Arc<AuditStore>,
    pub config: Arc<Config>,
    pub agent_signer: Arc<AgentSigner>,
    pub kora_gateway: Arc<KoraGateway>,
    pub orca_client: Arc<OrcaClient>,
}
