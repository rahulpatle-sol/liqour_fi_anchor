use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    self, Mint, TokenAccount, TokenInterface, Transfer,
};

pub mod constants;
pub mod error;
pub mod state;

use constants::*;
use error::LiqourError;
use state::{UserVault, VaultConfig};

declare_id!("FGJS4S51o9rSvxeomGrqacdwPFnZbBuU6p9KzhRHUx3b");

#[program]
pub mod liqour_defi {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let cfg = &mut ctx.accounts.vault_config;
        cfg.authority = ctx.accounts.authority.key();
        cfg.usdc_mint = ctx.accounts.usdc_mint.key();
        cfg.vault_token_account = ctx.accounts.vault_token_account.key();
        cfg.total_deposited = 0;
        cfg.bump = ctx.bumps.vault_config;
        msg!("Liqour vault initialized!");
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        require!(amount > 0, LiqourError::ZeroAmount);

        token_interface::transfer(
            CpiContext::new(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: ctx.accounts.user_usdc.to_account_info(),
                    to: ctx.accounts.vault_token_account.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount,
        )?;

        ctx.accounts.vault_config.total_deposited = ctx
            .accounts
            .vault_config
            .total_deposited
            .checked_add(amount)
            .ok_or(LiqourError::Overflow)?;

        let uv = &mut ctx.accounts.user_vault;
        uv.owner = ctx.accounts.user.key();
        uv.deposited = uv.deposited.checked_add(amount).ok_or(LiqourError::Overflow)?;
        uv.bump = ctx.bumps.user_vault;

        msg!("Deposited {} micro-USDC", amount);
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        require!(amount > 0, LiqourError::ZeroAmount);
        require!(
            ctx.accounts.vault_token_account.amount >= amount,
            LiqourError::InsufficientVaultBalance
        );

        let bump = ctx.accounts.vault_config.bump;
        let seeds: &[&[u8]] = &[VAULT_CONFIG_SEED, &[bump]];

        token_interface::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: ctx.accounts.vault_token_account.to_account_info(),
                    to: ctx.accounts.user_usdc.to_account_info(),
                    authority: ctx.accounts.vault_config.to_account_info(),
                },
                &[seeds],
            ),
            amount,
        )?;

        ctx.accounts.vault_config.total_deposited =
            ctx.accounts.vault_config.total_deposited.saturating_sub(amount);

        ctx.accounts.user_vault.withdrawn = ctx
            .accounts
            .user_vault
            .withdrawn
            .checked_add(amount)
            .ok_or(LiqourError::Overflow)?;

        msg!("Withdrew {} micro-USDC", amount);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    pub usdc_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = authority,
        token::mint = usdc_mint,
        token::authority = vault_config,
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init,
        payer = authority,
        space = VaultConfig::LEN,
        seeds = [VAULT_CONFIG_SEED],
        bump,
    )]
    pub vault_config: Account<'info, VaultConfig>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        constraint = user_usdc.owner == user.key(),
        constraint = user_usdc.mint == vault_config.usdc_mint,
    )]
    pub user_usdc: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        constraint = vault_token_account.key() == vault_config.vault_token_account,
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [VAULT_CONFIG_SEED],
        bump = vault_config.bump,
    )]
    pub vault_config: Account<'info, VaultConfig>,

    #[account(
        init_if_needed,
        payer = user,
        space = UserVault::LEN,
        seeds = [USER_VAULT_SEED, user.key().as_ref()],
        bump,
    )]
    pub user_vault: Account<'info, UserVault>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(
        constraint = authority.key() == vault_config.authority @ LiqourError::Unauthorized
    )]
    pub authority: Signer<'info>,

    #[account(mut)]
    pub user_usdc: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        constraint = vault_token_account.key() == vault_config.vault_token_account,
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [VAULT_CONFIG_SEED],
        bump = vault_config.bump,
    )]
    pub vault_config: Account<'info, VaultConfig>,

    #[account(
        mut,
        seeds = [USER_VAULT_SEED, user_usdc.owner.as_ref()],
        bump = user_vault.bump,
    )]
    pub user_vault: Account<'info, UserVault>,

    pub token_program: Interface<'info, TokenInterface>,
}
