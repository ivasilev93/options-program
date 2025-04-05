use std::str::FromStr;

use anchor_lang::prelude::*;
use anchor_spl::token_interface::{ TokenInterface, Mint, TokenAccount };

use crate::errors::*;
use crate::state::market::*;
use crate::constants::ADMIN_KEY;

#[derive(Accounts)]
#[instruction(
    name: String,
    fee: u64,
    ix: u64
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
    pub fn handle(ctx: Context<CreateMarket>, fee: u64, name: String, ix: u64) -> Result<()> {
        let market = &mut ctx.accounts.market;

        market.id = ix;
        market.name = name;
        market.fee = fee;
        market.bump = ctx.bumps.market;

        Ok(())
    }
}