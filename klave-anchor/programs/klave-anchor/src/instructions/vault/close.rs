use anchor_lang::prelude::*;

use crate::state::vault::AgentVault;

#[derive(Accounts)]
pub struct CloseVault<'info> {
    #[account(
        mut,
        close = agent,
        seeds = [b"vault", agent.key().as_ref()],
        bump = vault.bump,
        has_one = agent
    )]
    pub vault: Account<'info, AgentVault>,

    #[account(mut)]
    pub agent: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn vault_closure_handler(_ctx: Context<CloseVault>) -> Result<()> {
    // Rent is automatically recovered to the agent via the `close = agent` attribute
    Ok(())
}
