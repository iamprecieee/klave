use std::fmt;

use serde::{Deserialize, Serialize};

use crate::agent::model::AgentPolicy;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PolicyViolation {
    AgentInactive,
    ProgramNotAllowed(String),
    ExceedsMaxLamports { requested: i64, limit: i64 },
    TokenNotAllowed(String),
    DailySpendExceeded { current_usd: f64, limit_usd: f64 },
    DailySwapVolumeExceeded { current_usd: f64, limit_usd: f64 },
    SlippageExceeded { requested_bps: i32, limit_bps: i32 },
    WithdrawalDestinationNotAllowed(String),
}

impl fmt::Display for PolicyViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AgentInactive => write!(f, "agent is inactive"),
            Self::ProgramNotAllowed(p) => write!(f, "program not in allowlist: {p}"),
            Self::ExceedsMaxLamports { requested, limit } => {
                write!(f, "requested {requested} lamports exceeds limit of {limit}")
            }
            Self::TokenNotAllowed(m) => write!(f, "token not in allowlist: {m}"),
            Self::DailySpendExceeded {
                current_usd,
                limit_usd,
            } => write!(
                f,
                "daily spend {current_usd} USD would exceed limit of {limit_usd} USD"
            ),
            Self::DailySwapVolumeExceeded {
                current_usd,
                limit_usd,
            } => write!(
                f,
                "daily swap volume {current_usd} USD would exceed limit of {limit_usd} USD"
            ),
            Self::SlippageExceeded {
                requested_bps,
                limit_bps,
            } => write!(
                f,
                "requested slippage {requested_bps} bps exceeds policy limit of {limit_bps} bps"
            ),
            Self::WithdrawalDestinationNotAllowed(d) => {
                write!(f, "withdrawal destination not in allowlist: {d}")
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstructionType {
    SolTransfer,
    InitializeVault,
    DepositToVault,
    WithdrawFromVault,
    TokenSwap,
    AgentWithdrawal,
    CloseVault,
}

impl fmt::Display for InstructionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SolTransfer => write!(f, "sol_transfer"),
            Self::InitializeVault => write!(f, "initialize_vault"),
            Self::DepositToVault => write!(f, "deposit_to_vault"),
            Self::WithdrawFromVault => write!(f, "withdraw_from_vault"),
            Self::TokenSwap => write!(f, "token_swap"),
            Self::AgentWithdrawal => write!(f, "agent_withdrawal"),
            Self::CloseVault => write!(f, "close_vault"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransactionRequest {
    pub instruction_type: InstructionType,
    pub lamports: Option<i64>,
    pub program_ids: Vec<String>,
    pub mints: Vec<String>,
    pub destination: Option<String>,
    pub slippage_bps: Option<i32>,
    pub is_active: bool,
}

pub struct PolicyEngine;

impl PolicyEngine {
    pub fn evaluate(
        policy: &AgentPolicy,
        request: &TransactionRequest,
        daily_spend_usd: f64,
    ) -> Result<(), Vec<PolicyViolation>> {
        let mut violations = Vec::new();

        if !request.is_active {
            violations.push(PolicyViolation::AgentInactive);
        }
        for pid in &request.program_ids {
            if !policy.allowed_programs.contains(pid) {
                violations.push(PolicyViolation::ProgramNotAllowed(pid.clone()));
            }
        }

        if let Some(lamports) = request.lamports
            && lamports > policy.max_lamports_per_tx
        {
            violations.push(PolicyViolation::ExceedsMaxLamports {
                requested: lamports,
                limit: policy.max_lamports_per_tx,
            });
        }

        for mint in &request.mints {
            if !policy.token_allowlist.contains(mint) {
                violations.push(PolicyViolation::TokenNotAllowed(mint.clone()));
            }
        }

        if policy.daily_spend_limit_usd > 0.0 && daily_spend_usd > policy.daily_spend_limit_usd {
            violations.push(PolicyViolation::DailySpendExceeded {
                current_usd: daily_spend_usd,
                limit_usd: policy.daily_spend_limit_usd,
            });
        }

        if let Some(requested_bps) = request.slippage_bps
            && requested_bps > policy.slippage_bps
        {
            violations.push(PolicyViolation::SlippageExceeded {
                requested_bps,
                limit_bps: policy.slippage_bps,
            });
        }

        if let Some(ref dest) = request.destination
            && matches!(
                request.instruction_type,
                InstructionType::AgentWithdrawal | InstructionType::SolTransfer
            )
            && (policy.withdrawal_destinations.is_empty()
                || !policy.withdrawal_destinations.contains(dest))
        {
            violations.push(PolicyViolation::WithdrawalDestinationNotAllowed(
                dest.clone(),
            ));
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }

    pub fn check_swap_static(
        policy: &AgentPolicy,
        input_mint: &str,
        output_mint: &str,
        slippage_bps: i32,
    ) -> Result<(), Vec<PolicyViolation>> {
        let mut violations = Vec::new();

        if !policy.token_allowlist.contains(&input_mint.to_string()) {
            violations.push(PolicyViolation::TokenNotAllowed(input_mint.to_string()));
        }

        if !policy.token_allowlist.contains(&output_mint.to_string()) {
            violations.push(PolicyViolation::TokenNotAllowed(output_mint.to_string()));
        }

        if slippage_bps > policy.slippage_bps {
            violations.push(PolicyViolation::SlippageExceeded {
                requested_bps: slippage_bps,
                limit_bps: policy.slippage_bps,
            });
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }

    pub fn check_swap_volume(
        policy: &AgentPolicy,
        quote_usd_value: f64,
        daily_swap_volume_usd: f64,
    ) -> Result<(), Vec<PolicyViolation>> {
        let mut violations = Vec::new();

        if policy.daily_swap_volume_usd > 0.0 {
            let projected_volume = daily_swap_volume_usd + quote_usd_value;
            if projected_volume > policy.daily_swap_volume_usd {
                violations.push(PolicyViolation::DailySwapVolumeExceeded {
                    current_usd: projected_volume,
                    limit_usd: policy.daily_swap_volume_usd,
                });
            }
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }
}
