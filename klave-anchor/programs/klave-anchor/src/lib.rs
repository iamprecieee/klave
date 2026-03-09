pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use instructions::*;

declare_id!("3nKoeBAeLjcePc7pJPfdZpohsAbUR7U7pJ3HztovbyFx");

#[program]
pub mod klave_anchor {
    use super::*;

    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        instructions::vault_init_handler(ctx)
    }

    pub fn deposit(ctx: Context<VaultOperation>, amount: u64) -> Result<()> {
        instructions::vault_deposit_handler(ctx, amount)
    }

    pub fn withdraw(ctx: Context<VaultOperation>, amount: u64) -> Result<()> {
        instructions::vault_withdraw_handler(ctx, amount)
    }

    pub fn close_vault(ctx: Context<CloseVault>) -> Result<()> {
        instructions::vault_closure_handler(ctx)
    }
}
