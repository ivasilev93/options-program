use anchor_lang::prelude::*;

mod state;
mod errors;
mod math;
mod constants;
mod instructions;
mod common;

use instructions::admin::{ market_create:: *, market_update_vol::*, withdraw_fees::*, market_close::* };
use instructions::makers::{ market_deposit::*, market_withdraw::* };
use instructions::takers::{ acc_create::*, buy::*, exercise::* };

declare_id!("Be2AgTUf5uVfdHaSXPpzifVkmwfkgRwtLToVywevfvrS");

#[program]
pub mod options_program {

    use super::*;

    // --- Admin --- ///
    pub fn create_market(ctx: Context<CreateMarket>, params: CreateMarketParams) -> Result<()> {
        CreateMarket::handle(ctx, params)
    }

    pub fn update_market_vol(ctx: Context<UpdateMarketVol>, params: UpdateMarketVolParams) -> Result<()> {
        UpdateMarketVol::handle(ctx, params)
    }  

    pub fn withdraw_fees(ctx: Context<WithdrawFees>, params: WithdrawFeesParams) -> Result<()> {
        WithdrawFees::handle(ctx, params)
    }  

    pub fn close_market(ctx: Context<CloseMarket>, params: CloseMarketParams) -> Result<()> {
        CloseMarket::handle(ctx, params)
    }

    //TODO
    // Admin to pause market
    // Instruction for off-chain service to exercise option on expiry on user's behalf (for convenience)

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
