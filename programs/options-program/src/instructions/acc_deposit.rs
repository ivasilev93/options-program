use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenAccount;

use super::super::state::user_account::*;

#[derive(Accounts)]
#[instruction(amount_in_lamports: u64, market_id: u8)]
pub struct AccountDeposit<'info> {

    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(
        mut,
        seeds = [
            USR_ACC_SEED.as_bytes(),
            signer.key().as_ref()
        ],
        bump
    )]
    pub account: Account<'info, UserAccount>,

    #[account(
        mut,
        token::authority = signer,
        constraint = &user_token_acc.mint.eq(&market_vault.mint)
    )]
    pub user_token_acc: InterfaceAccount<'info, TokenAccount>,

    #[account(
        // mut, to test if can receive money
        seeds = [
            MARKET_SEED.as_bytes(),
            market_id.to_le_bytes().as_ref()
        ],
        bump
    )]  
    pub market_vault: InterfaceAccount<'info, TokenAccount>,
    
    pub system_program: Program<'info, System>
}

impl AccountDeposit<'_> {
    pub fn handle(ctx: Context<AccountDeposit>, amount_in_lamports: u64) -> Result<()> {
        
        let account = &mut ctx.accounts.account;
        

        msg!("Updated account {} balance to {}", account.key(), account.balance);

        

        Ok(())
    }
}