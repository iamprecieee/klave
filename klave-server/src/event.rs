use klave_core::agent::model::TokenBalance;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerEvent {
    AgentCreated {
        id: String,
        label: String,
    },
    AgentUpdated {
        id: String,
    },
    TransactionExecuted {
        signature: String,
        agent_id: String,
    },
    BalanceUpdated {
        agent_id: String,
        sol_lamports: u64,
        vault_lamports: u64,
        tokens: Vec<TokenBalance>,
    },
    Message {
        text: String,
    },
}

impl ServerEvent {
    /// Used for per-agent SSE filtering.
    pub fn agent_id(&self) -> Option<&str> {
        match self {
            ServerEvent::AgentCreated { id, .. } => Some(id),
            ServerEvent::AgentUpdated { id } => Some(id),
            ServerEvent::TransactionExecuted { agent_id, .. } => Some(agent_id),
            ServerEvent::BalanceUpdated { agent_id, .. } => Some(agent_id),
            ServerEvent::Message { .. } => None,
        }
    }
}
