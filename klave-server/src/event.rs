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
