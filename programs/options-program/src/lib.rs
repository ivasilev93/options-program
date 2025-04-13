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

    //Admin
    pub fn create_market(ctx: Context<CreateMarket>, fee: u64, name: String, ix: u16, price_feed: String, volatility_bps: u32) -> Result<()> {
        CreateMarket::handle(ctx, fee, name, ix, price_feed, volatility_bps)
    }
    //Exercise option ? by cron

    //Takers (Option buyers)
    pub fn create_account(ctx: Context<AccountCreate>) -> Result<()> {
        AccountCreate::handle(ctx)
    }

    pub fn buy(ctx: Context<BuyOption>, params: BuyOptionParams) -> Result<()> {
        BuyOption::handle(ctx, params)
    }

    pub fn exercise(ctx: Context<ExerciseOption>, market_ix: u16, option_id: u8) -> Result<()> {
        ExerciseOption::handle(ctx, market_ix, option_id)
    }

    //Liquidity providers (LPs)
    pub fn market_deposit(ctx: Context<MarketDeposit>, params: DepositIx) -> Result<()> {
        MarketDeposit::handle(ctx, params.amount, params.min_amount_out, params.ix)
    }
    pub fn market_withdraw(ctx: Context<MarketWithdraw>, params: WithdrawParams) -> Result<()> {
        MarketWithdraw::handle(ctx, params)
    }

   


    
}
