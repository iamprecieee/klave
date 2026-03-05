mod common;

use solana_sdk::signature::{Keypair, Signer};

#[tokio::test]
async fn test_deposit_increases_vault_balance() {
    let (mut banks_client, payer, _) = common::klave_program().start().await;

    let agent = Keypair::new();
    let (vault_pda, _) = common::agent_vault_pda(&agent.pubkey());

    let fund_ix = solana_sdk::system_instruction::transfer(
        &payer.pubkey(),
        &agent.pubkey(),
        2_000_000_000, // 2 SOL
    );
    common::send_transaction(&mut banks_client, &[fund_ix], &payer, &[&payer])
        .await
        .unwrap();

    let init_ix = common::initialize_vault_ix(vault_pda, &agent, &payer);
    common::send_transaction(&mut banks_client, &[init_ix], &payer, &[&payer, &agent])
        .await
        .unwrap();

    let balance_before = banks_client
        .get_account(vault_pda)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    let deposit_amount = 500_000_000; // 0.5 SOL
    let deposit_ix = common::vault_deposit_ix(vault_pda, &agent, deposit_amount);
    common::send_transaction(&mut banks_client, &[deposit_ix], &agent, &[&agent])
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
        balance_before + deposit_amount,
        "vault balance should increase by the deposited amount"
    );
}

#[tokio::test]
async fn test_deposit_fails_without_agent_signature() {
    let (mut banks_client, payer, _) = common::klave_program().start().await;

    let agent = Keypair::new();
    let (vault_pda, _) = common::agent_vault_pda(&agent.pubkey());

    let init_ix = common::initialize_vault_ix(vault_pda, &agent, &payer);
    common::send_transaction(&mut banks_client, &[init_ix], &payer, &[&payer, &agent])
        .await
        .unwrap();

    // attempt to deposit without the agent signing
    let deposit_ix = common::vault_deposit_ix(vault_pda, &agent, 100_000);
    let result =
        common::send_transaction(&mut banks_client, &[deposit_ix], &payer, &[&payer]).await;

    assert!(
        result.is_err(),
        "deposit without agent signature should fail"
    );
}
