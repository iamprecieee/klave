use anchor_lang::prelude::*;

use crate::state::vault::AgentVault;

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 1, // discriminator + pubkey + bump
        seeds = [b"vault", agent.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, AgentVault>,

    pub agent: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn vault_init_handler(ctx: Context<InitializeVault>) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    vault.agent = ctx.accounts.agent.key();
    vault.bump = ctx.bumps.vault;
    Ok(())
}
