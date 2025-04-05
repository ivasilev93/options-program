use anchor_lang::prelude::*;

mod state;
mod errors;
mod constants;
mod instructions;

use instructions::takers::acc_create::*;
use instructions::admin::market_create::*;

declare_id!("Be2AgTUf5uVfdHaSXPpzifVkmwfkgRwtLToVywevfvrS");

#[program]
pub mod options_program {
    use super::*;

    /* Admin */

    pub fn create_market(ctx: Context<CreateMarket>, fee: u64, name: String, ix: u64) -> Result<()> {
        CreateMarket::handle(ctx, fee, name, ix)
    }
    //Exercise option ? by cron

    //Taker
    pub fn create_account(ctx: Context<AccountCreate>) -> Result<()> {
        AccountCreate::handle(ctx)
    }
    
    //Buy option

    //Exercise option - by option holder

    //LPs

    //Deposit into market(pool)
    //Withdraw from market - for makers

   


    
}
