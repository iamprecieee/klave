use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

use crate::{error::VaultError, state::vault::AgentVault};

#[derive(Accounts)]
pub struct VaultOperation<'info> {
    #[account(
        mut,
        seeds = [b"vault", agent.key().as_ref()],
        bump = vault.bump,
        has_one = agent
    )]
    pub vault: Account<'info, AgentVault>,

    #[account(mut)]
    pub agent: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn vault_deposit_handler(ctx: Context<VaultOperation>, amount: u64) -> Result<()> {
    let cpi_context = CpiContext::new(
        ctx.accounts.system_program.to_account_info(),
        Transfer {
            from: ctx.accounts.agent.to_account_info(),
            to: ctx.accounts.vault.to_account_info(),
        },
    );
    transfer(cpi_context, amount)
}

pub fn vault_withdraw_handler(ctx: Context<VaultOperation>, amount: u64) -> Result<()> {
    let vault_info = ctx.accounts.vault.to_account_info();
    let agent_info = ctx.accounts.agent.to_account_info();

    let rent = Rent::get()?;
    let min_rent = rent.minimum_balance(vault_info.data_len());

    if vault_info.lamports() < amount {
        return err!(VaultError::InsufficientFunds);
    }

    if vault_info.lamports() - amount < min_rent {
        return err!(VaultError::BelowRentExemptionThreshold);
    }

    **vault_info.try_borrow_mut_lamports()? -= amount;
    **agent_info.try_borrow_mut_lamports()? += amount;

    Ok(())
}
