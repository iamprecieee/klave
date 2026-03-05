#![allow(dead_code)]

use anchor_lang::{solana_program::system_program, InstructionData, ToAccountMetas};
use solana_program_test::ProgramTest;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

pub fn klave_program() -> ProgramTest {
    ProgramTest::new("klave_anchor", klave_anchor::ID, None)
}

pub fn agent_vault_pda(agent: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"vault", agent.as_ref()], &klave_anchor::ID)
}

pub fn initialize_vault_ix(vault: Pubkey, agent: &Keypair, payer: &Keypair) -> Instruction {
    Instruction {
        program_id: klave_anchor::ID,
        accounts: klave_anchor::accounts::InitializeVault {
            vault,
            agent: agent.pubkey(),
            payer: payer.pubkey(),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: klave_anchor::instruction::InitializeVault {}.data(),
    }
}

pub fn vault_deposit_ix(vault: Pubkey, agent: &Keypair, amount: u64) -> Instruction {
    Instruction {
        program_id: klave_anchor::ID,
        accounts: klave_anchor::accounts::VaultOperation {
            vault,
            agent: agent.pubkey(),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: klave_anchor::instruction::Deposit { amount }.data(),
    }
}

pub fn vault_withdraw_ix(vault: Pubkey, agent: &Keypair, amount: u64) -> Instruction {
    Instruction {
        program_id: klave_anchor::ID,
        accounts: klave_anchor::accounts::VaultOperation {
            vault,
            agent: agent.pubkey(),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: klave_anchor::instruction::Withdraw { amount }.data(),
    }
}

pub fn close_vault_ix(vault: Pubkey, agent: &Keypair) -> Instruction {
    Instruction {
        program_id: klave_anchor::ID,
        accounts: klave_anchor::accounts::CloseVault {
            vault,
            agent: agent.pubkey(),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: klave_anchor::instruction::CloseVault {}.data(),
    }
}

pub async fn send_transaction(
    banks_client: &mut solana_program_test::BanksClient,
    instructions: &[Instruction],
    fee_payer: &Keypair,
    signers: &[&Keypair],
) -> Result<(), solana_program_test::BanksClientError> {
    let blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut tx = Transaction::new_with_payer(instructions, Some(&fee_payer.pubkey()));
    let _ = tx.try_sign(signers, blockhash);
    banks_client.process_transaction(tx).await
}
