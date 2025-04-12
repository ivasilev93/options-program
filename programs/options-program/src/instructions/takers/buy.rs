use std::str::FromStr;

use anchor_lang::prelude::*;
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};
use anchor_spl::token_interface::{self, *};
use crate::{common::OptionType, constants::CALL_MULTIPLIER, errors::CustomError, state::{event::OptionBought, market::*, user_account::{self, *}}};

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct BuyOptionParams {
    pub market_ix: u16,
    pub option: OptionType,
    pub strike_price_usd: u64, //strike price in usd scaled by 6 decimals (e.g. for $120 -> 120_000_000)
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
        token::mint = asset_mint
    )]
    pub user_token_acc: InterfaceAccount<'info, TokenAccount>,

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

    #[account(
        mut,
        seeds = [
            PROTOCOL_FEES_VAULT_SEED.as_bytes(),
            params.market_ix.to_le_bytes().as_ref()
        ],
        bump,
    )]
    pub protocol_fees_vault: InterfaceAccount<'info, TokenAccount>,

    #[account()]
    pub asset_mint: InterfaceAccount<'info, Mint>,

    #[account()]
    pub price_update: Account<'info, PriceUpdateV2>,
    pub token_program: Interface<'info, TokenInterface>,
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
        // require!(market.price_feed == ctx.accounts.price_update.key(), CustomError::OrdersLimitExceeded);  this is not correct. on program address for pyth, multiple feed ids

        //Get asset price from oracle in usd, scaled by 10^6 (Pyth)
        let price_update = &mut ctx.accounts.price_update;
        let maximum_age: u64 = 60;
        let feed_id = get_feed_id_from_hex(market.price_feed.as_str())?;
        // let feed_id: [u8; 32] = get_feed_id_from_hex(bytes)?;
        let price = price_update.get_price_no_older_than(&Clock::get()?, maximum_age, &feed_id)?;

        let token_scaling = 10_u64.pow(market.asset_decimals as u32);

        //check if market has enough collateral to support options exercises
        let max_potential_payout_in_tokens = match params.option {
            OptionType::CALL => {
                let usd_payout = (params.strike_price_usd as u128)
                .checked_mul(CALL_MULTIPLIER as u128).unwrap()
                .checked_mul(params.quantity as u128).unwrap();

                (usd_payout * token_scaling as u128 / price.price as u128) as u64
            },
            OptionType::PUT => {
                let usd_payout = (params.strike_price_usd as u128)
                .checked_mul(params.quantity as u128).unwrap();
                (usd_payout * token_scaling as u128 / price.price as u128) as u64
            }
        };

        let available_collateral = market.reserve_supply - market.committed_reserve;
        require!(available_collateral > max_potential_payout_in_tokens, CustomError::InsufficientColateral);        

        //Calculate premium
        let seconds_per_year: f64 = 365.25 * 24.0 * 60.0 * 60.0;
        let asset_price_usd = price.price as u64; 
        let strike_price_usd = params.strike_price_usd;
        let time_to_expire_in_years = time_distance as f64 / seconds_per_year;
        let volatility = market.volatility_bps as f64 / 1000.0; // Not optimal solution. Just for demo simplicity.

        let premium_amount = calculate_premium(
            strike_price_usd,
            asset_price_usd,
            time_to_expire_in_years,
            volatility, 
            &params.option,
            market.asset_decimals)?;

        let protocol_fee = (premium_amount * market.fee_bps) / 10_000;
        let lp_share = premium_amount - protocol_fee;

        //Transfer premium to market vault
        token_interface::transfer_checked(
            CpiContext::new(ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.user_token_acc.to_account_info(),
                to: ctx.accounts.market_vault.to_account_info(),
                authority: ctx.accounts.signer.to_account_info(),
                mint: ctx.accounts.asset_mint.to_account_info()
            }),
            premium_amount,
            ctx.accounts.asset_mint.decimals)?;

        let market_ix_bytes = params.market_ix.to_le_bytes();
        //Transfer protocol fees to fee vault
        let signer_seeds: &[&[&[u8]]] = &[&[
            MARKET_VAULT_SEED.as_bytes(),
            market_ix_bytes.as_ref(),
            &[ctx.bumps.market_vault]]];

        token_interface::transfer_checked(
            CpiContext::new(ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.market_vault.to_account_info(),
                to: ctx.accounts.protocol_fees_vault.to_account_info(),
                authority: ctx.accounts.market_vault.to_account_info(),
                mint: ctx.accounts.asset_mint.to_account_info()
            }).with_signer(signer_seeds),
            protocol_fee,
            ctx.accounts.asset_mint.decimals)?;

        market.premiums = market.premiums.checked_add(lp_share).ok_or(CustomError::Overflow)?;
        market.committed_reserve = market.committed_reserve.checked_add(max_potential_payout_in_tokens).ok_or(CustomError::Overflow)?;

        //Save user option
        user_account.options[slot_ix] = OptionOrder {
            expiry: params.expiry_stamp,
            market_ix: params.market_ix,
            option_type: params.option.clone(),
            premium: premium_amount,
            strike_price: params.strike_price_usd
        };

        msg!("Option has been bought: 
        market: {}
        created_stamp: {}
        expiry_stamp: {}
        max_potential_payout_in_tokens: {}
        quantity: {}
        strike_price_usd: {}
        option: {:?}
        user: {}
        ",
        params.market_ix,
        stamp_now,
        params.expiry_stamp,
        max_potential_payout_in_tokens,
        params.quantity,
        params.strike_price_usd,
        params.option.clone(),
        ctx.accounts.signer.key());

        //Emit event
        emit!(OptionBought {
            market: params.market_ix,
            created_stamp: stamp_now,
            expiry_stamp: params.expiry_stamp,
            max_potential_payout_in_tokens: max_potential_payout_in_tokens,
            quantity: params.quantity,
            strike_price_usd: params.strike_price_usd,
            option: params.option.clone(),
            user: ctx.accounts.signer.key()
        });

        Ok(())
    }
}