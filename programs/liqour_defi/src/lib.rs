use anchor_lang::prelude::*;


pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use instructions::deposit::Deposit;
use instructions::initialize::Initialize;
use instructions::withdraw::Withdraw;

declare_id!("FGJS4S51o9rSvxeomGrqacdwPFnZbBuU6p9KzhRHUx3b");

#[program]
pub mod liqour_defi {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        instructions::initialize::initialize(ctx)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::deposit::deposit(ctx, amount)
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        instructions::withdraw::withdraw(ctx, amount)
    }
}