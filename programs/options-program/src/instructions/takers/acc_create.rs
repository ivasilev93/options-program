use anchor_lang::prelude::*;
use crate::state::user_account::*;

#[derive(Accounts)]
pub struct AccountCreate<'info> {

    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(
        init,
        payer = signer,
        seeds = [
            USR_ACC_SEED.as_bytes(),
            signer.key().as_ref()
        ],
        bump,
        space = 8 + UserAccount::INIT_SPACE
    )]
    pub account: AccountLoader<'info, UserAccount>,
    pub system_program: Program<'info, System>
}

impl AccountCreate<'_> {
    pub fn handle(_ctx: Context<AccountCreate>) -> Result<()> {
        Ok(())
    }
}