use serde::{Deserialize, Serialize};
use std::fmt;

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
    /// Evaluates the transaction request against the agent's policy.
    /// Returns `Ok(())` if the request passes all checks, or `Err(violations)`
    /// with every failing check collected.
    ///
    /// `daily_spend_usd` is the running total for today, queried from the audit
    /// store by the caller before invoking this method.
    pub fn evaluate(
        policy: &AgentPolicy,
        request: &TransactionRequest,
        daily_spend_usd: f64,
    ) -> Result<(), Vec<PolicyViolation>> {
        let mut violations = Vec::new();

        // 1. Agent must be active
        if !request.is_active {
            violations.push(PolicyViolation::AgentInactive);
        }

        // 2. All program IDs must be in the allowed list (empty list = deny all)
        for pid in &request.program_ids {
            if !policy.allowed_programs.contains(pid) {
                violations.push(PolicyViolation::ProgramNotAllowed(pid.clone()));
            }
        }

        // 3. Lamport cost within limit
        if let Some(lamports) = request.lamports
            && lamports > policy.max_lamports_per_tx
        {
            violations.push(PolicyViolation::ExceedsMaxLamports {
                requested: lamports,
                limit: policy.max_lamports_per_tx,
            });
        }

        // 4. Token mints must be in allowlist (empty list = deny all)
        for mint in &request.mints {
            if !policy.token_allowlist.contains(mint) {
                violations.push(PolicyViolation::TokenNotAllowed(mint.clone()));
            }
        }

        // 5. Daily spend limit (0 = unlimited)
        if policy.daily_spend_limit_usd > 0.0 && daily_spend_usd > policy.daily_spend_limit_usd {
            violations.push(PolicyViolation::DailySpendExceeded {
                current_usd: daily_spend_usd,
                limit_usd: policy.daily_spend_limit_usd,
            });
        }

        // 6. Slippage cap (swap-specific)
        if let Some(requested_bps) = request.slippage_bps
            && requested_bps > policy.slippage_bps
        {
            violations.push(PolicyViolation::SlippageExceeded {
                requested_bps,
                limit_bps: policy.slippage_bps,
            });
        }

        // 7. Withdrawal destination allowlist
        if let Some(ref dest) = request.destination
            && matches!(request.instruction_type, InstructionType::AgentWithdrawal)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::model::AgentPolicy;

    fn test_policy() -> AgentPolicy {
        AgentPolicy {
            id: "policy-1".to_string(),
            agent_id: "agent-1".to_string(),
            allowed_programs: vec!["prog1".to_string(), "prog2".to_string()],
            max_lamports_per_tx: 1_000_000,
            token_allowlist: vec!["mint_a".to_string(), "mint_b".to_string()],
            daily_spend_limit_usd: 100.0,
            daily_swap_volume_usd: 500.0,
            slippage_bps: 50,
            withdrawal_destinations: vec!["dest1".to_string()],
            updated_at: 0,
        }
    }

    fn base_request() -> TransactionRequest {
        TransactionRequest {
            instruction_type: InstructionType::SolTransfer,
            lamports: Some(500_000),
            program_ids: vec!["prog1".to_string()],
            mints: vec![],
            destination: None,
            slippage_bps: None,
            is_active: true,
        }
    }

    #[test]
    fn test_pass_valid_request() {
        let policy = test_policy();
        let req = base_request();
        assert!(PolicyEngine::evaluate(&policy, &req, 0.0).is_ok());
    }

    #[test]
    fn test_reject_inactive_agent() {
        let policy = test_policy();
        let mut req = base_request();
        req.is_active = false;
        let violations = PolicyEngine::evaluate(&policy, &req, 0.0).unwrap_err();
        assert!(violations.contains(&PolicyViolation::AgentInactive));
    }

    #[test]
    fn test_reject_unauthorized_program() {
        let policy = test_policy();
        let mut req = base_request();
        req.program_ids = vec!["unknown_prog".to_string()];
        let violations = PolicyEngine::evaluate(&policy, &req, 0.0).unwrap_err();
        assert!(matches!(
            violations[0],
            PolicyViolation::ProgramNotAllowed(_)
        ));
    }

    #[test]
    fn test_reject_exceeds_lamports() {
        let policy = test_policy();
        let mut req = base_request();
        req.lamports = Some(2_000_000);
        let violations = PolicyEngine::evaluate(&policy, &req, 0.0).unwrap_err();
        assert!(matches!(
            violations[0],
            PolicyViolation::ExceedsMaxLamports { .. }
        ));
    }

    #[test]
    fn test_reject_token_not_allowed() {
        let policy = test_policy();
        let mut req = base_request();
        req.mints = vec!["unknown_mint".to_string()];
        let violations = PolicyEngine::evaluate(&policy, &req, 0.0).unwrap_err();
        assert!(matches!(violations[0], PolicyViolation::TokenNotAllowed(_)));
    }

    #[test]
    fn test_reject_daily_spend_exceeded() {
        let policy = test_policy();
        let req = base_request();
        let violations = PolicyEngine::evaluate(&policy, &req, 150.0).unwrap_err();
        assert!(matches!(
            violations[0],
            PolicyViolation::DailySpendExceeded { .. }
        ));
    }

    #[test]
    fn test_reject_slippage_exceeded() {
        let policy = test_policy();
        let mut req = base_request();
        req.slippage_bps = Some(100);
        let violations = PolicyEngine::evaluate(&policy, &req, 0.0).unwrap_err();
        assert!(matches!(
            violations[0],
            PolicyViolation::SlippageExceeded { .. }
        ));
    }

    #[test]
    fn test_reject_withdrawal_destination() {
        let policy = test_policy();
        let mut req = base_request();
        req.instruction_type = InstructionType::AgentWithdrawal;
        req.destination = Some("attacker_address".to_string());
        let violations = PolicyEngine::evaluate(&policy, &req, 0.0).unwrap_err();
        assert!(matches!(
            violations[0],
            PolicyViolation::WithdrawalDestinationNotAllowed(_)
        ));
    }

    #[test]
    fn test_pass_valid_withdrawal_destination() {
        let policy = test_policy();
        let mut req = base_request();
        req.instruction_type = InstructionType::AgentWithdrawal;
        req.destination = Some("dest1".to_string());
        assert!(PolicyEngine::evaluate(&policy, &req, 0.0).is_ok());
    }

    #[test]
    fn test_unlimited_daily_spend() {
        let mut policy = test_policy();
        policy.daily_spend_limit_usd = 0.0;
        let req = base_request();
        // Even with high spend, 0 = unlimited
        assert!(PolicyEngine::evaluate(&policy, &req, 999_999.0).is_ok());
    }

    #[test]
    fn test_collects_multiple_violations() {
        let policy = test_policy();
        let mut req = base_request();
        req.is_active = false;
        req.lamports = Some(2_000_000);
        req.program_ids = vec!["bad_prog".to_string()];
        let violations = PolicyEngine::evaluate(&policy, &req, 0.0).unwrap_err();
        assert!(violations.len() >= 3);
    }

    #[test]
    fn test_empty_allowlists_reject_everything() {
        let mut policy = test_policy();
        policy.allowed_programs = vec![];
        policy.token_allowlist = vec![];

        let mut req = base_request();
        req.program_ids = vec!["any_prog".to_string()];
        req.mints = vec!["any_mint".to_string()];

        let violations = PolicyEngine::evaluate(&policy, &req, 0.0).unwrap_err();
        assert!(violations.contains(&PolicyViolation::ProgramNotAllowed("any_prog".to_string())));
        assert!(violations.contains(&PolicyViolation::TokenNotAllowed("any_mint".to_string())));
    }
}
