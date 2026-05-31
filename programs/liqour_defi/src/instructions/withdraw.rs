// programs/liqour_defi/src/instructions/withdraw.rs
use anchor_lang::prelude::*;
// FIX: use token:: NOT token_interface::
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::{constants::*, error::LiqourError, state::{VaultConfig, UserVault}};

pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    require!(amount > 0, LiqourError::ZeroAmount);
    require!(
        ctx.accounts.vault_token_account.amount >= amount,
        LiqourError::InsufficientVaultBalance
    );

    let bump = ctx.accounts.vault_config.bump;
    let seeds: &[&[u8]] = &[VAULT_CONFIG_SEED, &[bump]];

    // vault_config PDA signs the transfer
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from:      ctx.accounts.vault_token_account.to_account_info(),
                to:        ctx.accounts.user_usdc.to_account_info(),
                authority: ctx.accounts.vault_config.to_account_info(),
            },
            &[seeds],
        ),
        amount,
    )?;

    ctx.accounts.vault_config.total_deposited =
        ctx.accounts.vault_config.total_deposited.saturating_sub(amount);

    ctx.accounts.user_vault.withdrawn = ctx
        .accounts.user_vault.withdrawn
        .checked_add(amount)
        .ok_or(LiqourError::Overflow)?;

    msg!("Withdrew {} micro-USDC", amount);
    Ok(())
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    /// Only authority (backend wallet) can call this
    #[account(
        constraint = authority.key() == vault_config.authority @ LiqourError::Unauthorized
    )]
    pub authority: Signer<'info>,

    /// User's USDC — receives funds
    #[account(mut)]
    pub user_usdc: Account<'info, TokenAccount>,

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

    #[account(
        mut,
        seeds = [USER_VAULT_SEED, user_usdc.owner.as_ref()],
        bump  = user_vault.bump,
    )]
    pub user_vault: Account<'info, UserVault>,

    pub token_program: Program<'info, Token>,
}
