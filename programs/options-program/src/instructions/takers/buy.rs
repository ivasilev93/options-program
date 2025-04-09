use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenAccount;
use crate::{common::OptionType, constants::CALL_MULTIPLIER, errors::CustomError, state::{market::*, user_account::{self, *}}};

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct BuyOptionParams {
    pub market_ix: u16,
    pub option: OptionType,
    pub strike_price_usd: u64,
    pub expiry_stamp: i64,
    pub quantity: u64
}

#[derive(Accounts)]
#[instruction(params: BuyOptionParams)]
pub struct BuyOption<'info> {
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
        seeds = [
            MARKET_SEED.as_bytes(),
            params.market_ix.to_le_bytes().as_ref()
        ],
        bump,
    )]
    pub market: Account<'info, Market>,

    #[account(
        mut,
        seeds = [
            MARKET_VAULT_SEED.as_bytes(),
            params.market_ix.to_le_bytes().as_ref()
        ],
        bump,
    )]
    pub market_vault: InterfaceAccount<'info, TokenAccount>,

}

impl BuyOption<'_> {
    pub fn handle(ctx: Context<BuyOption>, params: BuyOptionParams) -> Result<()> {

        let user_account = &mut ctx.accounts.account;
        let market = &mut ctx.accounts.market;

        //Check avaiable slots in array
        let slot_ix = user_account.get_available_slot(params.market_ix)
            .ok_or(CustomError::OrdersLimitExceeded)?;

        let stamp_now = Clock::get()?.unix_timestamp;
        let time_distance = params.expiry_stamp - stamp_now;
        let seconds_in_day: i64 = 86400;
        require!(time_distance > 0, CustomError::InvalidExpiry);
        require!(time_distance / seconds_in_day <= 30, CustomError::InvalidExpiry);

        //check if market has enough collateral to support options exercises
        let max_potential_payout = match params.option {
            OptionType::CALL => {
                params.strike_price_usd
                .checked_mul(CALL_MULTIPLIER).unwrap()
                .checked_mul(params.quantity).unwrap()
            },
            OptionType::PUT => {
                params.strike_price_usd
                .checked_mul(params.quantity).unwrap()
            }
        };

        let available_collateral = market.reserve_supply - market.committed_reserve;
        require!(available_collateral > max_potential_payout, CustomError::InsufficientColateral);

        //Oracle check

        //Calculate premium
        let seconds_per_year: f64 = 365.25 * 24.0 * 60.0 * 60.0;
        let asset_price_usd = 0; //Get oracle price in usd
        let strike_price_usd = params.strike_price_usd;
        let time_to_expire_in_years = time_distance as f64 / seconds_per_year;

        //Transfer premium to vault

        //Subtrakt protocol fee from premium

        //Safe user option

        //Emit event

        Ok(())
    }
}