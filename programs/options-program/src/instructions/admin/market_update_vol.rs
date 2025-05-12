use std::str::FromStr;
use anchor_lang::prelude::*;
use crate::errors::*;
use crate::state::market::*;
use crate::constants::ADMIN_KEY;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UpdateMarketVolParams {
    pub ix: u16, 
    pub hour1_volatility_bps: u32,
    pub hour4_volatility_bps: u32,
    pub day1_volatility_bps: u32,
    pub day3_volatility_bps: u32,
    pub week_volatility_bps: u32
}

#[derive(Accounts)]
#[instruction(params: UpdateMarketVolParams)]
pub struct UpdateMarketVol<'info> {
    #[account(
        mut,
        constraint = signer.key() == Pubkey::from_str(ADMIN_KEY).unwrap() @ CustomError::Unauthorized
    )]
    pub signer: Signer<'info>,

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

    pub system_program: Program<'info, System>
}

impl UpdateMarketVol<'_> {
    pub fn handle(ctx: Context<UpdateMarketVol>, params: UpdateMarketVolParams) -> Result<()> {
        let market = &mut ctx.accounts.market;

        market.hour1_volatility_bps = params.hour1_volatility_bps;
        market.hour4_volatility_bps = params.hour4_volatility_bps;
        market.day1_volatility_bps = params.day1_volatility_bps;
        market.day3_volatility_bps = params.day3_volatility_bps;
        market.week_volatility_bps = params.week_volatility_bps;

        let clock = Clock::get()?;
        market.vol_last_updated = clock.unix_timestamp;

        msg!("Market {} updated. Vol data: 1H: {}, 4H: {}, 1D: {}, 3D: {}, 1W: {}", 
            market.id, 
            params.hour1_volatility_bps, 
            params.hour4_volatility_bps, 
            params.day1_volatility_bps, 
            params.day3_volatility_bps, 
            params.week_volatility_bps);

        Ok(())
    }
}