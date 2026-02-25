use anchor_client::{
    solana_sdk::{
        self, commitment_config::CommitmentConfig, pubkey::Pubkey, signature::read_keypair_file,
        signature::Keypair, signer::Signer,
    },
    Client, Cluster,
};
use klave_anchor::accounts::{InitializeVault, VaultOperation};
use klave_anchor::instruction::{
    Deposit as DepositInstruction, InitializeVault as InitializeVaultInstruction,
    Withdraw as WithdrawInstruction,
};
use std::str::FromStr;

#[test]
fn test_treasury_flow() {
    let program_id = "GCU8h2yUZKPKemrxGu4tZoiiiUdhWeSonaWCgYbZaRBx";
    let anchor_wallet = std::env::var("ANCHOR_WALLET").unwrap_or_else(|_| {
        let home = std::env::var("HOME").expect("HOME not set");
        format!("{home}/.config/solana/id.json")
    });
    let payer = read_keypair_file(&anchor_wallet).unwrap();

    let client = Client::new_with_options(Cluster::Localnet, &payer, CommitmentConfig::confirmed());
    let program_id = Pubkey::from_str(program_id).unwrap();
    let program = client.program(program_id).unwrap();

    let agent = Keypair::new();

    // Airdrop some SOL to the agent so it can deposit
    let tx = program
        .request()
        .instruction(solana_sdk::system_instruction::transfer(
            &payer.pubkey(),
            &agent.pubkey(),
            10_000_000_000,
        ))
        .send()
        .unwrap();
    println!("Funded agent: {}", tx);

    let (vault_pda, _bump) =
        Pubkey::find_program_address(&[b"vault", agent.pubkey().as_ref()], &program_id);

    // Initialize Vault
    let tx = program
        .request()
        .accounts(InitializeVault {
            vault: vault_pda,
            agent: agent.pubkey(),
            payer: payer.pubkey(),
            system_program: anchor_client::solana_sdk::system_program::ID,
        })
        .args(InitializeVaultInstruction {})
        .send()
        .expect("Initialize should succeed");

    println!("Initialized vault: {}", tx);

    // Deposit
    let tx = program
        .request()
        .accounts(VaultOperation {
            vault: vault_pda,
            agent: agent.pubkey(),
            system_program: anchor_client::solana_sdk::system_program::ID,
        })
        .args(DepositInstruction { amount: 1_000_000 })
        .signer(&agent)
        .send()
        .expect("Deposit should succeed");

    println!("Deposited 0.001 SOL: {}", tx);

    // Withdraw
    let tx = program
        .request()
        .accounts(VaultOperation {
            vault: vault_pda,
            agent: agent.pubkey(),
            system_program: anchor_client::solana_sdk::system_program::ID,
        })
        .args(WithdrawInstruction { amount: 500_000 })
        .signer(&agent)
        .send()
        .expect("Withdraw should succeed");

    println!("Withdrew 0.0005 SOL: {}", tx);
}
