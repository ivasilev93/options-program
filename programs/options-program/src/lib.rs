use anchor_lang::prelude::*;

mod state;
mod errors;
mod constants;
mod instructions;
mod common;

use instructions::takers::{ acc_create::*, buy::* };
use instructions::makers::market_deposit::*;
use instructions::admin::market_create::*;

declare_id!("Be2AgTUf5uVfdHaSXPpzifVkmwfkgRwtLToVywevfvrS");

#[program]
pub mod options_program {
    use super::*;

    /* Admin */

    pub fn create_market(ctx: Context<CreateMarket>, fee: u64, name: String, ix: u16, price_feed: String, volatility_bps: u32) -> Result<()> {
        CreateMarket::handle(ctx, fee, name, ix, price_feed, volatility_bps)
    }
    //Exercise option ? by cron

    //Taker
    pub fn create_account(ctx: Context<AccountCreate>) -> Result<()> {
        AccountCreate::handle(ctx)
    }
    
    //Buy option
    pub fn buy(ctx: Context<BuyOption>, params: BuyOptionParams) -> Result<()> {
        BuyOption::handle(ctx, params)
    }

    //Exercise option - by option holder

    //LPs

    pub fn market_deposit(ctx: Context<MarketDeposit>, params: DepositIx) -> Result<()> {
        MarketDeposit::handle(ctx, params.amount, params.min_amount_out, params.ix)
    }
    
    //Withdraw from market - for makers

   


    
}
