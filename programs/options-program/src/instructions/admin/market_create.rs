use std::str::FromStr;

use anchor_lang::prelude::*;
use anchor_spl::token_interface::{ TokenInterface, Mint, TokenAccount };

use crate::errors::*;
use crate::state::market::*;
use crate::constants::ADMIN_KEY;

#[derive(Accounts)]
#[instruction(
    fee: u64,
    name: String,
    ix: u16,
    price_feed: String
)]
pub struct CreateMarket<'info> {
    #[account(
        mut,
        constraint = signer.key() == Pubkey::from_str(ADMIN_KEY).unwrap() @ CustomError::Unauthorized
    )]
    pub signer: Signer<'info>,

    #[account()]
    pub asset_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = signer,
        seeds = [
            MARKET_LP_MINT_SEED.as_bytes(),
            ix.to_le_bytes().as_ref(),
        ],
        mint::decimals = asset_mint.decimals,
        mint::authority = lp_mint.key(),
        mint::freeze_authority = lp_mint.key(),
        bump
    )]
    pub lp_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = signer,
        seeds = [
            MARKET_SEED.as_bytes(),
            ix.to_le_bytes().as_ref(),
            // signer.key().as_ref()
        ],
        bump,
        space = 8 + Market::INIT_SPACE
    )]
    pub market: Account<'info, Market>,

    #[account(
        init,
        payer = signer,
        token::mint = asset_mint,
        token::authority = market_vault,
        token::token_program = token_program,
        seeds = [
            MARKET_VAULT_SEED.as_bytes(),
            ix.to_le_bytes().as_ref(),
            // signer.key().as_ref()
        ],
        bump,
    )]
    pub market_vault: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>
}

impl CreateMarket<'_> {
    pub fn handle(ctx: Context<CreateMarket>, fee: u64, name: String, ix: u16, price_feed: String) -> Result<()> {
        let market = &mut ctx.accounts.market;
        let market_acc_info = market.to_account_info();

        let price_feed_pubkey = Pubkey::from_str(&price_feed).unwrap();
        market.id = ix;
        market.name = name;
        market.fee_bps = fee;
        market.bump = ctx.bumps.market;
        market.price_feed = price_feed_pubkey;

        msg!("Market seeds: {:?} {:?}", MARKET_SEED.as_bytes(), ix.to_le_bytes());
        msg!("Market address: {} ", market_acc_info.key());

        Ok(())
    }
}