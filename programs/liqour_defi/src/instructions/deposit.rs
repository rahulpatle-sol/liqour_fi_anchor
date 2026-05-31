// programs/liqour_defi/src/instructions/deposit.rs
use anchor_lang::prelude::*;
// FIX: use token:: NOT token_interface::
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::{constants::*, error::LiqourError, state::{VaultConfig, UserVault}};

pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    require!(amount > 0, LiqourError::ZeroAmount);

    // SPL transfer: user_usdc → vault_token_account
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from:      ctx.accounts.user_usdc.to_account_info(),
                to:        ctx.accounts.vault_token_account.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount,
    )?;

    // Update vault total
    ctx.accounts.vault_config.total_deposited = ctx
        .accounts.vault_config.total_deposited
        .checked_add(amount)
        .ok_or(LiqourError::Overflow)?;

    // Update user vault
    let uv   = &mut ctx.accounts.user_vault;
    uv.owner     = ctx.accounts.user.key();
    uv.deposited = uv.deposited.checked_add(amount).ok_or(LiqourError::Overflow)?;
    uv.bump      = ctx.bumps.user_vault;

    msg!("Deposited {} micro-USDC", amount);
    Ok(())
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// User's USDC token account
    #[account(
        mut,
        constraint = user_usdc.owner == user.key(),
        constraint = user_usdc.mint  == vault_config.usdc_mint,
    )]
    pub user_usdc: Account<'info, TokenAccount>,

    /// Vault USDC token account
    #[account(
        mut,
        constraint = vault_token_account.key() == vault_config.vault_token_account,
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [VAULT_CONFIG_SEED],
        bump  = vault_config.bump,
    )]
    pub vault_config: Account<'info, VaultConfig>,

    /// Per-user record — init_if_needed needs feature flag in Cargo.toml
    #[account(
        init_if_needed,
        payer  = user,
        space  = UserVault::LEN,
        seeds  = [USER_VAULT_SEED, user.key().as_ref()],
        bump,
    )]
    pub user_vault: Account<'info, UserVault>,

    pub token_program:  Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent:           Sysvar<'info, Rent>,
}
