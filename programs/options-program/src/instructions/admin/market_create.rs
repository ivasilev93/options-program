use std::str::FromStr;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{ TokenInterface, Mint, TokenAccount };
use crate::errors::*;
use crate::state::market::*;
use crate::constants::ADMIN_KEY;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CreateMarketParams {
    pub fee: u64, 
    pub name: String, 
    pub ix: u16, 
    pub price_feed: String, 
    pub volatility_bps: u32
}

#[derive(Accounts)]
#[instruction(params: CreateMarketParams)]
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
            params.ix.to_le_bytes().as_ref(),
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
            params.ix.to_le_bytes().as_ref()
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
            params.ix.to_le_bytes().as_ref()
        ],
        bump,
    )]
    pub market_vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init,
        payer = signer,
        token::mint = asset_mint,
        token::authority = protocol_fees_vault,
        token::token_program = token_program,
        seeds = [
            PROTOCOL_FEES_VAULT_SEED.as_bytes(),
            params.ix.to_le_bytes().as_ref()
        ],
        bump,
    )]
    pub protocol_fees_vault: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>
}

impl CreateMarket<'_> {
    pub fn handle(ctx: Context<CreateMarket>, params: CreateMarketParams) -> Result<()> {
        let market = &mut ctx.accounts.market;
        let asset_mint = &mut ctx.accounts.asset_mint;
        let market_acc_info = market.to_account_info();

        market.id = params.ix;
        market.name = params.name;
        market.fee_bps = params.fee;
        market.bump = ctx.bumps.market;
        market.price_feed = params.price_feed;
        market.asset_decimals = asset_mint.decimals;
        market.volatility_bps = params.volatility_bps;
        market.asset_mint = asset_mint.key();

        msg!("Market seeds: {:?} {:?}", MARKET_SEED.as_bytes(), params.ix.to_le_bytes());
        msg!("Market address: {} ", market_acc_info.key());

        Ok(())
    }
}