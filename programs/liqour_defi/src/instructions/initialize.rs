// programs/liqour_defi/src/instructions/initialize.rs
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::{constants::*, state::VaultConfig};

pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
    let cfg = &mut ctx.accounts.vault_config;
    cfg.authority           = ctx.accounts.authority.key();
    cfg.usdc_mint           = ctx.accounts.usdc_mint.key();
    cfg.vault_token_account = ctx.accounts.vault_token_account.key();
    cfg.total_deposited     = 0;
    cfg.bump                = ctx.bumps.vault_config;
    msg!("Liqour vault initialized!");
    Ok(())
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    pub usdc_mint: Account<'info, Mint>,

    #[account(
        init,
        payer  = authority,
        token::mint      = usdc_mint,
        token::authority = vault_config,
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(
        init,
        payer  = authority,
        space  = VaultConfig::LEN,
        seeds  = [VAULT_CONFIG_SEED],
        bump,
    )]
    pub vault_config: Account<'info, VaultConfig>,

    pub token_program:  Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent:           Sysvar<'info, Rent>,
}
