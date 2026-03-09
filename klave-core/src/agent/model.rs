use serde::{Deserialize, Serialize};

pub const SYSTEM_PROGRAM_ID: &str = "11111111111111111111111111111111";
pub const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
pub const TREASURY_PROGRAM_ID: &str = "3nKoeBAeLjcePc7pJPfdZpohsAbUR7U7pJ3HztovbyFx";
pub const ORCA_WHIRLPOOL_PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub pubkey: String,
    pub label: String,
    pub is_active: bool,
    pub created_at: i64,
    pub policy_id: String,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPolicy {
    pub id: String,
    pub agent_id: String,
    pub allowed_programs: Vec<String>,
    pub max_lamports_per_tx: i64,
    pub token_allowlist: Vec<String>,
    pub daily_spend_limit_usd: f64,
    pub daily_swap_volume_usd: f64,
    pub slippage_bps: i32,
    pub withdrawal_destinations: Vec<String>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAgentRequest {
    pub label: String,
    pub policy: AgentPolicyInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPolicyInput {
    pub allowed_programs: Option<Vec<String>>,
    pub max_lamports_per_tx: Option<i64>,
    pub token_allowlist: Option<Vec<String>>,
    pub daily_spend_limit_usd: Option<f64>,
    pub daily_swap_volume_usd: Option<f64>,
    pub slippage_bps: Option<i32>,
    pub withdrawal_destinations: Option<Vec<String>>,
}

pub fn default_programs() -> Vec<String> {
    vec![
        SYSTEM_PROGRAM_ID.to_string(),
        TOKEN_PROGRAM_ID.to_string(),
        TREASURY_PROGRAM_ID.to_string(),
        ORCA_WHIRLPOOL_PROGRAM_ID.to_string(),
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBalance {
    pub sol_lamports: u64,
    pub vault_lamports: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
    pub mint: String,
    pub amount: u64,
    pub decimals: u8,
    pub ui_amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapQuote {
    pub input_amount: u64,
    pub output_amount: u64,
    pub min_output_amount: u64,
    pub price_impact_bps: u64,
    pub fee_amount: u64,
}
