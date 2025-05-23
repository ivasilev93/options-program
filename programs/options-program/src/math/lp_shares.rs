use anchor_lang::prelude::*;
use core::{cmp::min};
use crate::{errors::CustomError, state::market::Market};

/// Calculates the amount of LP tokens to mint when adding liquidity to the market.
/// LP tokens to mint are calculated as a proportion of existing LP tokens based on the deposit's share of total market value.
/// 
/// @param base_asset_amount - Amount of base asset being deposited
/// 
/// @param min_amount_out - Minimum LP tokens expected to receive (slippage protection)
/// 
/// @param market - Reference to the market where liquidity is being added
/// 
/// @returns Result<u64> - Amount of LP tokens to mint on success, scaled in base units, or error
pub fn calc_lp_shares(base_asset_amount: u64, min_amount_out: u64, market: &Market) -> Result<u64> {
    require!(base_asset_amount > 0, CustomError::InvalidAmount);
    require!(min_amount_out > 0, CustomError::InvalidAmount);

    let market_tvl = market.premiums.checked_add(market.reserve_supply).unwrap();

    let lp_tokens_to_mint = if market.lp_minted == 0 {
        base_asset_amount * 1_000
    } else {
        let scale = 1_000_000_000 as u64;

        let scaled_asset = (base_asset_amount as u128)
            .checked_mul(scale as u128).unwrap()
            .checked_div(market_tvl as u128).unwrap();
        // println!("scaled asset {:?}", scaled_asset);

        let lp_tokens = scaled_asset
            .checked_mul(market.lp_minted as u128).unwrap()
            .checked_div(scale as u128).unwrap();
        // println!("lp_tokens {:?}", lp_tokens);

        let lp_tokens_u64 = lp_tokens.try_into().map_err(|_| CustomError::Overflow)?;

        require!(lp_tokens_u64 >= 1, CustomError::DustAmount);
        lp_tokens_u64
    };

    require!(lp_tokens_to_mint >= min_amount_out, CustomError::SlippageExceeded);

    Ok(lp_tokens_to_mint)
}

/// Calculates the amount of base assets to withdraw based on LP tokens being burned, 
/// accounting for the proportion of total liquidity owned and ensuring withdrawal amounts 
/// don't exceed available uncommitted reserves.
pub fn calc_withdraw_amount_from_lp_shares(lp_tokens_to_burn: u64, market: &Market,) -> Result<(u64, u64)> {
    require!(lp_tokens_to_burn > 0, CustomError::InvalidAmount);
    require!(market.lp_minted >= lp_tokens_to_burn, CustomError::InsufficientShares);

    let scale = 1_000_000_000 as u64;

    let ownership_ratio = (lp_tokens_to_burn as u128)
        .checked_mul(scale as u128).unwrap()
        .checked_div(market.lp_minted as u128).unwrap();

    let market_tvl = market.reserve_supply
        .checked_add(market.premiums).unwrap();
    require!(market_tvl > 0, CustomError::InvalidState);

    let potential_withdraw_amount = ownership_ratio
        .checked_mul(market_tvl as u128).unwrap()
        .checked_div(scale as u128).unwrap() as u64;

    //Check if amount to be withdraw is not as collateral to unexercised options
    let uncomitted_reserve = market_tvl
        .checked_sub(market.committed_reserve).unwrap();
       
    let withdrawable_amount = min(uncomitted_reserve, potential_withdraw_amount);
    require!(withdrawable_amount >= 1, CustomError::CannotWithdraw);

    let actual_lp_tokens_to_burn = if withdrawable_amount < potential_withdraw_amount {
        ((withdrawable_amount as u128)
            .checked_mul(market.lp_minted as u128).unwrap()
            .checked_div(market_tvl as u128).unwrap()
        ) as u64
    } else {
        lp_tokens_to_burn
    };

    require!(actual_lp_tokens_to_burn > 0, CustomError::InvalidAmount);

    msg!("Requested tokens to burn: {}, max withdrawable: {}, actual tokens to burn: {}",
        lp_tokens_to_burn, withdrawable_amount, actual_lp_tokens_to_burn);

    Ok((withdrawable_amount, actual_lp_tokens_to_burn))
}