use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("Insufficient funds in vault")]
    InsufficientFunds,
    #[msg("Withdrawal would leave vault below rent-exemption threshold")]
    BelowRentExemptionThreshold,
}
