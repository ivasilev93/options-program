use anchor_lang::prelude::*;

mod state;
mod errors;
mod constants;
mod instructions;
mod common;

use instructions::takers::{ acc_create::*, buy::*, exercise::* };
use instructions::makers::market_deposit::*;
use instructions::makers::market_withdraw::*;
use instructions::admin::market_create::*;

declare_id!("Be2AgTUf5uVfdHaSXPpzifVkmwfkgRwtLToVywevfvrS");

#[program]
pub mod options_program {
    use super::*;

    // --- Admin --- ///
    pub fn create_market(ctx: Context<CreateMarket>, params: CreateMarketParams) -> Result<()> {
        CreateMarket::handle(ctx, params)
    }    
    //TODO:
    //Allow admin to withdraw protocol fees    
    //Expose instruction for authorized off-chain cron to exercise options on taker's behalf for convenience 

    // --- Takers (Option buyers) --- //
    pub fn create_account(ctx: Context<AccountCreate>) -> Result<()> {
        AccountCreate::handle(ctx)
    }
    pub fn buy(ctx: Context<BuyOption>, params: BuyOptionParams) -> Result<()> {
        BuyOption::handle(ctx, params)
    }
    pub fn exercise(ctx: Context<ExerciseOption>, params: ExerciseOptionParams) -> Result<()> {
        ExerciseOption::handle(ctx, params.market_ix, params.option_id)
    }

    // --- Liquidity providers (LPs) --- //
    pub fn market_deposit(ctx: Context<MarketDeposit>, params: DepositIx) -> Result<()> {
        MarketDeposit::handle(ctx, params.amount, params.min_amount_out, params.ix)
    }
    pub fn market_withdraw(ctx: Context<MarketWithdraw>, params: WithdrawParams) -> Result<()> {
        MarketWithdraw::handle(ctx, params)
    }
}
