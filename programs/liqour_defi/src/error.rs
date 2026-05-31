// programs/liqour_defi/src/error.rs
use anchor_lang::prelude::*;

#[error_code]
pub enum LiqourError {
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("Insufficient vault balance")]
    InsufficientVaultBalance,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Math overflow")]
    Overflow,
}
