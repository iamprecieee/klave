use std::sync::Arc;

use klave_core::agent::repository::AgentRepository;
use klave_core::audit::store::AuditStore;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub agent_repo: Arc<AgentRepository>,
    pub audit_store: Arc<AuditStore>,
    pub config: Arc<Config>,
}
