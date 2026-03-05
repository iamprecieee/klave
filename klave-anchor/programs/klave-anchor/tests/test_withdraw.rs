mod common;

use solana_program_test::*;
use solana_sdk::signature::{Keypair, Signer};

async fn setup_funded_vault(
    banks_client: &mut BanksClient,
    payer: &solana_sdk::signature::Keypair,
    agent: &Keypair,
    deposit_amount: u64,
) {
    let fund_ix =
        solana_sdk::system_instruction::transfer(&payer.pubkey(), &agent.pubkey(), 2_000_000_000);
    common::send_transaction(banks_client, &[fund_ix], payer, &[payer])
        .await
        .unwrap();

    let (vault_pda, _) = common::agent_vault_pda(&agent.pubkey());

    let init_ix = common::initialize_vault_ix(vault_pda, agent, payer);
    common::send_transaction(banks_client, &[init_ix], payer, &[payer, agent])
        .await
        .unwrap();

    let deposit_ix = common::vault_deposit_ix(vault_pda, agent, deposit_amount);
    common::send_transaction(banks_client, &[deposit_ix], agent, &[agent])
        .await
        .unwrap();
}

#[tokio::test]
async fn test_withdraw_decreases_vault_balance() {
    let (mut banks_client, payer, _) = common::klave_program().start().await;
    let agent = Keypair::new();
    let (vault_pda, _) = common::agent_vault_pda(&agent.pubkey());

    setup_funded_vault(&mut banks_client, &payer, &agent, 1_000_000_000).await;

    let balance_before = banks_client
        .get_account(vault_pda)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    let withdraw_amount = 100_000_000; // 0.1 SOL
    let withdraw_ix = common::vault_withdraw_ix(vault_pda, &agent, withdraw_amount);
    common::send_transaction(&mut banks_client, &[withdraw_ix], &agent, &[&agent])
        .await
        .unwrap();

    let balance_after = banks_client
        .get_account(vault_pda)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    assert_eq!(
        balance_after,
        balance_before - withdraw_amount,
        "vault balance should decrease by the withdrawn amount"
    );
}

#[tokio::test]
async fn test_withdraw_fails_when_amount_exceeds_balance() {
    let (mut banks_client, payer, _) = common::klave_program().start().await;
    let agent = Keypair::new();
    let (vault_pda, _) = common::agent_vault_pda(&agent.pubkey());

    setup_funded_vault(&mut banks_client, &payer, &agent, 100_000_000).await;

    let vault_balance = banks_client
        .get_account(vault_pda)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // try to withdraw more than the vault holds
    let withdraw_ix = common::vault_withdraw_ix(vault_pda, &agent, vault_balance + 1);
    let result =
        common::send_transaction(&mut banks_client, &[withdraw_ix], &agent, &[&agent]).await;

    assert!(
        result.is_err(),
        "withdrawing more than the vault balance should fail"
    );
}

#[tokio::test]
async fn test_withdraw_fails_when_vault_would_go_below_rent_exemption() {
    let (mut banks_client, payer, _) = common::klave_program().start().await;
    let agent = Keypair::new();
    let (vault_pda, _) = common::agent_vault_pda(&agent.pubkey());

    // deposit just enough to be rent exempt, not much more
    setup_funded_vault(&mut banks_client, &payer, &agent, 1_000_000).await;

    let vault_balance = banks_client
        .get_account(vault_pda)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // withdraw almost everything, leaving vault below rent exemption
    let withdraw_ix = common::vault_withdraw_ix(vault_pda, &agent, vault_balance - 1);
    let result =
        common::send_transaction(&mut banks_client, &[withdraw_ix], &agent, &[&agent]).await;

    assert!(
        result.is_err(),
        "withdrawal that drops vault below rent exemption should fail"
    );
}
