use core::{cmp::min};
use anchor_lang::prelude::*;

use crate::{common::OptionType, constants::*, errors::CustomError };

pub const MARKET_SEED: &str = "market";
pub const MARKET_VAULT_SEED: &str = "market_vault";
pub const PROTOCOL_FEES_VAULT_SEED: &str = "protocol_fees_vault";
pub const MARKET_LP_MINT_SEED: &str = "market_lp_mint";

#[account]
#[derive(InitSpace, PartialEq, Eq)]
pub struct Market {

    pub id: u16,
    #[max_len(32)]
    pub name: String,     
    pub asset_mint: Pubkey,   
    pub fee_bps: u64,             // 50 bps = 0.5%
    pub bump: u8,
    pub reserve_supply: u64,      // Token smallest units (e.g., 10^9 for SOL, 10^6 for JUP)
    pub committed_reserve: u64,   // Token smallest units
    pub premiums: u64,            // Token smallest units 
    pub lp_minted: u64,
    pub volatility_bps: u32,      // 1bps=0.01%. Set by admin for demo simplicity. In prod, this would require different impl.
    #[max_len(70)]
    pub price_feed: String,       // Pyth feed (TOKEN)/USD
    pub asset_decimals: u8
}

pub fn calculate_required_collateral(
    market: &Market,
    option: &OptionType,
    strike_price_scaled: u128,
    current_price_scaled: u128,
    quantity: u64
) -> Result<u64> {

    let required_collateral_usd = match option {
        OptionType::CALL => {
            // Simple approach
            if current_price_scaled > strike_price_scaled {
                // Already in the money: (current_price - strike_price) * quantity * 2
                current_price_scaled
                    .checked_sub(strike_price_scaled).unwrap()
                    .checked_mul(quantity as u128).unwrap()
                    .checked_mul(CALL_MULTIPLIER  as u128).unwrap()
            } else {
                // Not in the money yet      
                strike_price_scaled
                    .checked_sub(current_price_scaled).unwrap()
                    .checked_mul(quantity as u128).unwrap()
                    .checked_mul(CALL_MULTIPLIER  as u128).unwrap()            
            }
        },
        OptionType::PUT => {
            //Half of potential payout: (strike price * quantity) / 2
            strike_price_scaled
                .checked_mul(quantity as u128).unwrap()
                .checked_div(PUT_MULTIPLIER as u128).unwrap()
        }
    };
    
    // Convert USD amount to token amount
    // Price is in USD with 6 decimals, need to convert to token amount
    let token_decimals = 10u128.pow(market.asset_decimals as u32);
    
    // (USD amount * token decimals) / (USD per token)
    let required_collateral_tokens = required_collateral_usd
        .checked_mul(token_decimals).unwrap()
        .checked_div(current_price_scaled).unwrap();

    msg!("required_collateral_usd: {}", required_collateral_usd);
    msg!("token_decimals: {}", token_decimals);
    msg!("required_collateral_tokens: {}", required_collateral_tokens);
    msg!("market: {}", market.reserve_supply);
    
    // Ensure market has enough liquidity
    require!(
        market.reserve_supply.checked_sub(market.committed_reserve).unwrap() >= required_collateral_tokens as u64,
        CustomError::InsufficientColateral);
    
    Ok(required_collateral_tokens as u64)
}

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
        base_asset_amount
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

    let market_tvl = market.reserve_supply.checked_add(market.premiums).unwrap();
    require!(market_tvl > 0, CustomError::InvalidState);

    let potential_withdraw_amount = ownership_ratio
        .checked_mul(market_tvl as u128).unwrap()
        .checked_div(scale as u128).unwrap() as u64;

    let uncomitted_reserve = market.reserve_supply.checked_sub(market.committed_reserve).unwrap();
    let max_withdrawable = uncomitted_reserve.checked_add(market.premiums).unwrap();   
    let withdrawable_amount = min(max_withdrawable, potential_withdraw_amount);

    require!(withdrawable_amount >= 1, CustomError::DustAmount);

    let actual_lp_tokens_to_burn = if withdrawable_amount < potential_withdraw_amount {
        ((withdrawable_amount as u128)
            .checked_mul(market.lp_minted as u128).unwrap()
            .checked_div(market_tvl as u128).unwrap()
        ) as u64
    } else {
        lp_tokens_to_burn
    };

    require!(actual_lp_tokens_to_burn > 0, CustomError::InvalidAmount);

    Ok((withdrawable_amount, actual_lp_tokens_to_burn))
}

/// Calculates premium for a given market (asset), based on provided data. Risk free rate is assumed to be 0 for simplicity.    
/// 
/// @param strike_price_usd - strike price in usd
/// 
/// @param spot_price_usd - spot price in usd 
/// 
/// @Returns
/// The premium amount in token units (scaled by the asset decimals)
pub fn calculate_premium(
    strike_price_usd: f64,
    spot_price_usd: f64,
    time_to_expity: f64,
    volatility: f64,
    option_type: &OptionType,
    asset_decimals: u8
) -> Result<u64> {
    // Convert to f64 for calculations, adjusting for scale
    let s = spot_price_usd as f64;
    let k = strike_price_usd as f64;

    // Assumed risk-free rate of 0 - for simplicity

    let d1 = (s / k).ln() + (volatility * volatility / 2.0) * time_to_expity;
    let d1 = d1 / (volatility * time_to_expity.sqrt());
    let d2 = d1 - volatility * time_to_expity.sqrt();

    // Approximate N(x) using a simple polynomial (for demo purposes)
    // In production, use a lookup table or more precise approximation
    let n_d1 = approximate_normal_cdf(d1)?;
    let n_d2 = approximate_normal_cdf(d2)?;

    let premium = match option_type {
        OptionType::CALL => s * n_d1 - k * n_d2, 
        OptionType::PUT => {
            let n_neg_d2 = approximate_normal_cdf(-d2)?; // N(-d2)
            let n_neg_d1 = approximate_normal_cdf(-d1)?; // N(-d1)
            k * n_neg_d2 - s * n_neg_d1
        }
    };

    let usd_per_token = s;
    let premium_in_tokens = premium / usd_per_token;
    let token_scaling = 10_f64.powi(asset_decimals as i32);

    // Scale back to u64 (10^6)
    let premium_scaled = (premium_in_tokens * token_scaling) as u64;
    
    Ok(premium_scaled)
}

fn approximate_normal_cdf(x: f64) -> Result<f64> {
     // Simple approximation for N(x) (for demo)
    // Replace with a lookup table or better polynomial in production
    let t = 1.0 / (1.0 + 0.2316419 * x.abs());
    let d = 0.3989423 * (-x * x / 2.0).exp();
    let p = 1.0 - d * t * (0.31938153 + t * (-0.356563782 + t * (1.781477937 + t * (-1.821255978 + t * 1.330274429))));
    Ok(if x >= 0.0 { p } else { 1.0 - p })
}
