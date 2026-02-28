use crate::agent::model::AgentPolicy;
use crate::policy::engine::{InstructionType, PolicyEngine, PolicyViolation, TransactionRequest};

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

#[test]
fn test_reject_sol_transfer_destination() {
    let policy = test_policy();
    let mut req = base_request();
    req.instruction_type = InstructionType::SolTransfer;
    req.destination = Some("attacker_address".to_string());
    let violations = PolicyEngine::evaluate(&policy, &req, 0.0).unwrap_err();
    assert!(
        violations.contains(&PolicyViolation::WithdrawalDestinationNotAllowed(
            "attacker_address".to_string()
        ))
    );
}
