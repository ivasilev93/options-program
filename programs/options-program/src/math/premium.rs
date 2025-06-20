use core::{cmp::max};
use anchor_lang::prelude::*;
use crate::{common::*, errors::CustomError, state::market::Market, constants::*};

const PRECISION: u128 = 100_000_000;

// Calculate option premium using a simplified model suitable for on-chain execution
pub fn calculate_option_premium(
    strike_price_usd: u128,
    spot_price_usd: u128,
    expiry: Expiry,
    market: &Market,
    option_type: &OptionType,
    quantity: u64,
) -> Result<(u64, u64, u64)> {
    require!(quantity > 0, CustomError::InvalidQuantity);
    require!(strike_price_usd > 0, CustomError::InvalidStrikePrice);
    
    // Expiry window is converted to time_to_expiry in seconds, as fraction of the year.
    let time_to_expiry_seconds = expiry.to_seconds().unwrap() as u128;    
    let time_to_expiry = (time_to_expiry_seconds * PRECISION) / SECONDS_IN_YEAR;
    
    let volatility_bps = market.get_volatility(&expiry).unwrap() as u128;
    require!(volatility_bps > 0, CustomError::InvalidVolatility);
    
    // Volatility as a scaled integer (bps to decimal equivalent)
    let volatility = volatility_bps * 10_000;
    
    // Calculate premium based on option type
    let scaled_usd_premium = calculate_premium(
            spot_price_usd,
            strike_price_usd,
            time_to_expiry,
            volatility,
            option_type
        )?;
    
    let total_scaled_usd_premium = scaled_usd_premium
        .checked_mul(quantity).ok_or(CustomError::Overflow)?;
    require!(total_scaled_usd_premium > 0, CustomError::PremiumCalcError);

    let premium_in_tokens = 
        u64::try_from(
            (total_scaled_usd_premium as u128)
            .checked_mul(10_u128.pow(market.asset_decimals as u32)).unwrap()
            .checked_div(spot_price_usd).unwrap()
        )?;

    // Apply market fee
    let fee_tokens = 
        premium_in_tokens
        .checked_mul(market.fee_bps).unwrap()
        .checked_div(10_000).unwrap();
    
    Ok((total_scaled_usd_premium, premium_in_tokens, fee_tokens))
}

     
// Premium = Intrinsic Value + Time Value
// where:
// Intrinsic Value = How much option if worth if exercised now
// Time value - extra value from volatility and time to expiry
fn calculate_premium(
    current_price: u128,
    strike_price: u128,
    time_to_expiry: u128,  // In years, scaled by PRECISION
    volatility: u128,      // Annual volatility, scaled by PRECISION
    option: &OptionType
) -> Result<u64> {
    
    let intrinsic = match option {
        OptionType::CALL => {
            if current_price > strike_price {
                current_price - strike_price
            } else {
                0
            }
        },
        OptionType::PUT => {
             if strike_price > current_price {
                strike_price - current_price
            } else {
                0
            }
        }
    };
    
    // Time value approximation by simplified formula w integers - volatility * price * sqrt(time_to_expiry)
    
    // Calculate sqrt using integer approximation
    let time_sqrt = sqrt(time_to_expiry);
    
    // Time value component (extra value from volatility and time to expiry)
    let time_value = (current_price * volatility * time_sqrt) / (PRECISION * 10_000);
    
    // If deep out of money (current < 0.8 * strike), reduce premium (apply discount)
    let moneyness_factor = if current_price < (strike_price * 8) / 10 {
        // Out of the money discount
        (current_price * PRECISION) / strike_price
    } else {
        PRECISION
    };
    
    let time_value_adjusted = (time_value * moneyness_factor) / PRECISION;
    let premium_price = u64::try_from(intrinsic + time_value_adjusted)?;
    
    // Combined premium calculation
    Ok(premium_price)
}

pub fn calculate_collateral(
    strike_usd: u128,
    current_usd: u128,
    option: &OptionType,
    market: &Market,
    expiry: Expiry,
    quantity: u64,
) -> Result<(u64, u64)> {

     // 1. Intrinsic value
    let intristic_value = match option {
        OptionType::CALL => {
             if current_usd > strike_usd {
                current_usd - strike_usd
            } else {
                0
            }
        },
        OptionType::PUT => {
             if strike_usd > current_usd {
                strike_usd - current_usd
            } else {
                0
            }
        }
    };

    // 2. sqrt(time) in bps
    let seconds_to_expiry = expiry.to_seconds().unwrap();
    let time_bps = (seconds_to_expiry as u128 * 10_000) / SECONDS_IN_YEAR;
    let sqrt_time_bps = sqrt(time_bps * 1_000_000) / 1_000;

    // 3. Buffer: spot * vol * sqrt(time)
    let vol_bps = market.get_volatility(&expiry)?; 
    let buffer = current_usd
        .checked_mul(vol_bps as u128).unwrap()
        .checked_mul(sqrt_time_bps).unwrap() 
        .checked_div(100_000).unwrap();

    // 4. Min collateral 20% of spot
    let min_collateral = (current_usd * 20) / 100;

    // 5. Total
    let collateral = max(intristic_value + buffer, min_collateral);
    let total_collateral = collateral
        .checked_mul(quantity as u128).ok_or(CustomError::Overflow)?;

    let token_decimals = 10u128.pow(market.asset_decimals as u32);

    let total_collateral_tokens = total_collateral
        .checked_mul(token_decimals).ok_or(CustomError::Overflow)?
        .checked_div(current_usd).ok_or(CustomError::Overflow)?;

    Ok((
        u64::try_from(total_collateral)?, 
        u64::try_from(total_collateral_tokens)?
    ))
}


// From uniswap v2 - babylonian method (https://en.wikipedia.org/wiki/Methods_of_computing_square_roots#Babylonian_method)
fn sqrt(y: u128) -> u128 {
    if y > 3 {
        let mut z = y;
        let mut x = y / 2 + 1;

        while x < z {
            z = x;
            x = (y / x + x) / 2;            
        }

        z
    } else if y != 0 {
        1
    } else {
        0
    }
}

//Non deterministic sketch impl for ref
//TODO: Look into Kamino Lend's handling of fractions w Fraction crate - https://docs.rs/fraction/latest/fraction/

// Calculates premium for a given market (asset), based on provided data. Risk free rate is assumed to be 0 for simplicity.
// More suitable for European style options   
// 
// @param strike_price_usd - strike price in usd
//
// @param spot_price_usd - spot price in usd 
// 
// @Returns
// The premium amount in token units (scaled by the asset decimals)
// pub fn calculate_premium(
//     strike_price_usd: f64,
//     spot_price_usd: f64,
//     time_to_expity: f64,
//     volatility: f64,
//     option_type: &OptionType,
//     asset_decimals: u8
// ) -> Result<u64> {
//     // Convert to f64 for calculations, adjusting for scale
//     let s = spot_price_usd as f64;
//     let k = strike_price_usd as f64;

//     // Assumed risk-free rate of 0 - for simplicity

//     let d1 = (s / k).ln() + (volatility * volatility / 2.0) * time_to_expity;
//     let d1 = d1 / (volatility * time_to_expity.sqrt());
//     let d2 = d1 - volatility * time_to_expity.sqrt();

//     // Approximate N(x) using a simple polynomial (for demo purposes)
//     // In production, use a lookup table or more precise approximation
//     let n_d1 = approximate_normal_cdf(d1)?;
//     let n_d2 = approximate_normal_cdf(d2)?;

//     let premium = match option_type {
//         OptionType::CALL => s * n_d1 - k * n_d2, 
//         OptionType::PUT => {
//             let n_neg_d2 = approximate_normal_cdf(-d2)?; // N(-d2)
//             let n_neg_d1 = approximate_normal_cdf(-d1)?; // N(-d1)
//             k * n_neg_d2 - s * n_neg_d1
//         }
//     };

//     let usd_per_token = s;
//     let premium_in_tokens = premium / usd_per_token;
//     let token_scaling = 10_f64.powi(asset_decimals as i32);

//     // Scale back to u64 (10^6)
//     let premium_scaled = (premium_in_tokens * token_scaling) as u64;
    
//     Ok(premium_scaled)
// }

// // //Cumulative distribution function (CDF) for a standard normal distribution
// pub fn approximate_normal_cdf(x: f64) -> Result<f64> {
//      // Simple approximation for N(x) (for demo)
//     // Replace with a lookup table or better polynomial in production
//     let t = 1.0 / (1.0 + 0.2316419 * x.abs());
//     let d = 0.3989423 * (-x * x / 2.0).exp();
//     let p = 1.0 - d * t * (0.31938153 + t * (-0.356563782 + t * (1.781477937 + t * (-1.821255978 + t * 1.330274429))));
//     Ok(if x >= 0.0 { p } else { 1.0 - p })
// }