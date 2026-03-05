use anchor_lang::prelude::{
    account, borsh, AnchorDeserialize, AnchorSerialize, Discriminator, Pubkey,
};

#[account]
pub struct AgentVault {
    pub agent: Pubkey,
    pub bump: u8,
}
