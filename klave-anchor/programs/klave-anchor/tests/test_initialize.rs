mod common;

use anchor_lang::AccountDeserialize;
use solana_sdk::signature::{Keypair, Signer};

#[tokio::test]
async fn test_vault_is_created_for_the_correct_agent() {
    let (mut banks_client, payer, _) = common::klave_program().start().await;

    let agent = Keypair::new();
    let (vault_pda, _) = common::agent_vault_pda(&agent.pubkey());
    let ix = common::initialize_vault_ix(vault_pda, &agent, &payer);
    common::send_transaction(&mut banks_client, &[ix], &payer, &[&payer, &agent])
        .await
        .unwrap();

    let account = banks_client
        .get_account(vault_pda)
        .await
        .unwrap()
        .expect("vault account should exist after initialization");

    let vault =
        klave_anchor::state::vault::AgentVault::try_deserialize(&mut account.data.as_slice())
            .unwrap();

    assert_eq!(vault.agent, agent.pubkey());
}

#[tokio::test]
async fn test_vault_cannot_be_initialized_twice() {
    let (mut banks_client, payer, _) = common::klave_program().start().await;

    let agent = Keypair::new();
    let (vault_pda, _) = common::agent_vault_pda(&agent.pubkey());
    let ix = common::initialize_vault_ix(vault_pda, &agent, &payer);
    common::send_transaction(&mut banks_client, &[ix.clone()], &payer, &[&payer, &agent])
        .await
        .unwrap();

    let result =
        common::send_transaction(&mut banks_client, &[ix], &payer, &[&payer, &agent]).await;

    assert!(
        result.is_err(),
        "initializing the same vault twice should fail"
    );
}
