use std::sync::Arc;

use solana_keychain::Signer as KSigner;

use crate::{agent::repository::AgentRepository, error::KlaveError};

#[derive(Clone)]
pub struct AgentSigner {
    repo: Arc<AgentRepository>,
}

impl AgentSigner {
    pub fn new(repo: Arc<AgentRepository>) -> Self {
        Self { repo }
    }

    pub async fn load(&self, agent_id: &str) -> Result<KSigner, KlaveError> {
        let keypair_bytes = self.repo.get_keypair(agent_id).await?;
        let json_str = serde_json::to_string(&keypair_bytes).map_err(|e| {
            KlaveError::Internal(format!("Failed to serialize keypair bytes: {}", e))
        })?;

        let signer = KSigner::from_memory(&json_str)
            .map_err(|e| KlaveError::Internal(format!("Failed to load memory signer: {}", e)))?;

        Ok(signer)
    }
}
