use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerEvent {
    AgentCreated { id: String, label: String },
    AgentUpdated { id: String },
    TransactionExecuted { signature: String, agent_id: String },
    BalanceUpdated { agent_id: String },
    Message { text: String },
}
