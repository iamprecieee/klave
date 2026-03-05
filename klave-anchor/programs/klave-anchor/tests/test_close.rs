mod common;

use solana_sdk::signature::{Keypair, Signer};

#[tokio::test]
async fn test_vault_no_longer_exists_after_close() {
    let (mut banks_client, payer, _) = common::klave_program().start().await;

    let agent = Keypair::new();
    let (vault_pda, _) = common::agent_vault_pda(&agent.pubkey());

    let fund_ix =
        solana_sdk::system_instruction::transfer(&payer.pubkey(), &agent.pubkey(), 500_000_000);
    common::send_transaction(&mut banks_client, &[fund_ix], &payer, &[&payer])
        .await
        .unwrap();

    let init_ix = common::initialize_vault_ix(vault_pda, &agent, &payer);
    common::send_transaction(&mut banks_client, &[init_ix], &payer, &[&payer, &agent])
        .await
        .unwrap();

    let close_ix = common::close_vault_ix(vault_pda, &agent);
    common::send_transaction(&mut banks_client, &[close_ix], &agent, &[&agent])
        .await
        .unwrap();

    let account = banks_client.get_account(vault_pda).await.unwrap();

    assert!(
        account.is_none(),
        "vault account should not exist after it is closed"
    );
}

#[tokio::test]
async fn test_close_returns_lamports_to_agent() {
    let (mut banks_client, payer, _) = common::klave_program().start().await;

    let agent = Keypair::new();
    let (vault_pda, _) = common::agent_vault_pda(&agent.pubkey());

    let fund_ix =
        solana_sdk::system_instruction::transfer(&payer.pubkey(), &agent.pubkey(), 500_000_000);
    common::send_transaction(&mut banks_client, &[fund_ix], &payer, &[&payer])
        .await
        .unwrap();

    let init_ix = common::initialize_vault_ix(vault_pda, &agent, &payer);
    common::send_transaction(&mut banks_client, &[init_ix], &payer, &[&payer, &agent])
        .await
        .unwrap();

    let vault_lamports = banks_client
        .get_account(vault_pda)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    let agent_balance_before = banks_client
        .get_account(agent.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    let close_ix = common::close_vault_ix(vault_pda, &agent);
    common::send_transaction(&mut banks_client, &[close_ix], &agent, &[&agent])
        .await
        .unwrap();

    let agent_balance_after = banks_client
        .get_account(agent.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // agent balance should increase by at least the vault's lamports
    // (minus transaction fees from the close transaction)
    assert!(
        agent_balance_after > agent_balance_before,
        "agent should receive lamports back after closing the vault"
    );
    assert!(
        agent_balance_after >= agent_balance_before + vault_lamports - 10_000,
        "agent should receive approximately the vault's lamports back"
    );
}

#[tokio::test]
async fn test_close_fails_when_signer_is_not_the_agent() {
    let (mut banks_client, payer, _) = common::klave_program().start().await;

    let agent = Keypair::new();
    let attacker = Keypair::new();
    let (vault_pda, _) = common::agent_vault_pda(&agent.pubkey());

    let init_ix = common::initialize_vault_ix(vault_pda, &agent, &payer);
    common::send_transaction(&mut banks_client, &[init_ix], &payer, &[&payer, &agent])
        .await
        .unwrap();

    // attacker tries to close the agent's vault
    let close_ix = common::close_vault_ix(vault_pda, &attacker);
    let result =
        common::send_transaction(&mut banks_client, &[close_ix], &attacker, &[&attacker]).await;

    assert!(
        result.is_err(),
        "a non-agent signer should not be able to close someone else's vault"
    );
}
