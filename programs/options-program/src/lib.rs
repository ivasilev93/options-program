use anchor_lang::prelude::*;

mod state;
mod instructions;

use instructions::{acc_create::AccountCreate, acc_deposit::AccountDeposit};

declare_id!("Be2AgTUf5uVfdHaSXPpzifVkmwfkgRwtLToVywevfvrS");

#[program]
pub mod options_program {
    use super::*;

    /* Admin */
    //Pause market?


    pub fn create_account(ctx: Context<AccountCreate>) -> Result<()> {
        AccountCreate::handle(ctx)
    }
    
    pub fn deposit_account(ctx: Context<AccountDeposit>, amountInLamports: u64) -> Result<()> {
        AccountDeposit::handle(ctx, amountInLamports)
    }

    //Deposit into market(pool) - for makers

    //Create account

    //Deposit into user_acc

    //Withdraw from market - for makers
    
    //Buy option - for takers

    //Exercise option - by option holder


    
}
