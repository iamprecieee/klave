use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

declare_id!("H2RojwyiyJ9CqTPoP1SynmutevCfq7YGskwcoPj1C7Ex");

#[account]
pub struct AgentVault {
    pub agent: Pubkey,
    pub bump: u8,
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
pub struct VaultOperation<'info> {
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

#[program]
pub mod klave_anchor {
    use super::*;

    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.agent = ctx.accounts.agent.key();
        vault.bump = ctx.bumps.vault;
        Ok(())
    }

    pub fn deposit(ctx: Context<VaultOperation>, amount: u64) -> Result<()> {
        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.agent.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
            },
        );
        transfer(cpi_context, amount)
    }

    pub fn withdraw(ctx: Context<VaultOperation>, amount: u64) -> Result<()> {
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

    pub fn close_vault(_ctx: Context<CloseVault>) -> Result<()> {
        // Rent is automatically recovered to the agent via the `close = agent` attribute
        Ok(())
    }
}

#[error_code]
pub enum VaultError {
    #[msg("Insufficient funds in vault")]
    InsufficientFunds,
    #[msg("Withdrawal would leave vault below rent-exemption threshold")]
    BelowRentExemptionThreshold,
}
