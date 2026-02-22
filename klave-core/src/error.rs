use thiserror::Error;

#[derive(Error, Debug)]
pub enum KlaveError {
    #[error("agent not found: {0}")]
    AgentNotFound(String),

    #[error("agent is inactive: {0}")]
    AgentInactive(String),

    #[error("policy not found for agent: {0}")]
    PolicyNotFound(String),

    #[error("policy violation: {violations:?}")]
    PolicyViolation { violations: Vec<String> },

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("internal error: {0}")]
    Internal(String),

    #[error("RPC error: {0}")]
    RpcError(String),
}
