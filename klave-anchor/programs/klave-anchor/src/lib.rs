use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

declare_id!("GCU8h2yUZKPKemrxGu4tZoiiiUdhWeSonaWCgYbZaRBx");

#[program]
pub mod klave_anchor {
    use super::*;

    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.agent = ctx.accounts.agent.key();
        vault.bump = ctx.bumps.vault;
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.agent.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
            },
        );
        transfer(cpi_context, amount)
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        let vault = &ctx.accounts.vault;
        let agent_key = vault.agent;
        let bump = vault.bump;

        let signer_seeds: &[&[&[u8]]] = &[&[b"vault", agent_key.as_ref(), &[bump]]];

        let cpi_context = CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.agent.to_account_info(),
            },
            signer_seeds,
        );

        transfer(cpi_context, amount)
    }
}

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

    /// CHECK: The agent pubkey. Used as a seed for the PDA.
    pub agent: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(
        mut,
        seeds = [b"vault", agent.key().as_ref()],
        bump = vault.bump
    )]
    pub vault: Account<'info, AgentVault>,

    #[account(mut)]
    pub agent: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(
        mut,
        seeds = [b"vault", agent.key().as_ref()],
        bump = vault.bump
    )]
    pub vault: Account<'info, AgentVault>,

    #[account(mut)]
    pub agent: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[account]
pub struct AgentVault {
    pub agent: Pubkey,
    pub bump: u8,
}
