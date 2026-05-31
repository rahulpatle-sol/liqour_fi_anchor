// programs/liqour_defi/src/state.rs
use anchor_lang::prelude::*;

#[account]
#[derive(Default)]
pub struct VaultConfig {
    pub authority:           Pubkey,
    pub usdc_mint:           Pubkey,
    pub vault_token_account: Pubkey,
    pub total_deposited:     u64,
    pub bump:                u8,
}
impl VaultConfig {
    pub const LEN: usize = 8 + 32 + 32 + 32 + 8 + 1;
}

#[account]
#[derive(Default)]
pub struct UserVault {
    pub owner:     Pubkey,
    pub deposited: u64,
    pub withdrawn: u64,
    pub bump:      u8,
}
impl UserVault {
    pub const LEN: usize = 8 + 32 + 8 + 8 + 1;
}
