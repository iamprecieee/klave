use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub pubkey: String,
    pub label: String,
    pub is_active: bool,
    pub created_at: i64,
    pub policy_id: String,
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
    #[serde(default)]
    pub allowed_programs: Vec<String>,
    #[serde(default = "default_max_lamports")]
    pub max_lamports_per_tx: i64,
    #[serde(default)]
    pub token_allowlist: Vec<String>,
    #[serde(default)]
    pub daily_spend_limit_usd: f64,
    #[serde(default)]
    pub daily_swap_volume_usd: f64,
    #[serde(default = "default_slippage_bps")]
    pub slippage_bps: i32,
    #[serde(default)]
    pub withdrawal_destinations: Vec<String>,
}

fn default_max_lamports() -> i64 {
    1_000_000_000 // 1 SOL
}

fn default_slippage_bps() -> i32 {
    50 // 0.5%
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
