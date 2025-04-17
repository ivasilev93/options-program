use core::cmp::min;
use anchor_lang::prelude::*;

use crate::{common::OptionType, errors::CustomError};

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
    pub fee_bps: u64, // 50 bps = 0.5%
    pub bump: u8,
    pub reserve_supply: u64,      // Token smallest units (e.g., 10^9 for SOL, 10^6 for JUP)
    pub committed_reserve: u64,   // Token smallest units...
    pub premiums: u64,            // Token smallest units 
    pub lp_minted: u64,
    pub volatility_bps: u32,      // 1bps=0.01%. Set by admin for demo simplicity. In prod, this would require different impl.
    #[max_len(70)]
    pub price_feed: String, // Pyth feed (TOKEN)/USD
    pub asset_decimals: u8
}

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
        println!("scaled asset {:?}", scaled_asset);

        let lp_tokens = scaled_asset
            .checked_mul(market.lp_minted as u128).unwrap()
            .checked_div(scale as u128).unwrap();
        println!("lp_tokens {:?}", lp_tokens);

        let lp_tokens_u64 = lp_tokens.try_into().map_err(|_| CustomError::Overflow)?;

        require!(lp_tokens_u64 >= 1, CustomError::DustAmount);
        lp_tokens_u64
    };

    require!(lp_tokens_to_mint >= min_amount_out, CustomError::SlippageExceeded);

    Ok(lp_tokens_to_mint)
}

pub fn calc_withdraw_amount_from_lp_shares(lp_tokens_to_burn: u64, market: &Market,) -> Result<(u64, u64)> {
    require!(lp_tokens_to_burn > 0, CustomError::InvalidAmount);
    require!(market.lp_minted > lp_tokens_to_burn, CustomError::InsufficientShares);

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

    //Other Solvency checks;
    require!(actual_lp_tokens_to_burn > 0, CustomError::InvalidAmount);

    Ok((withdrawable_amount, actual_lp_tokens_to_burn))
}

///Calculates premium for a given market (asset), based on provided data 
///  #Arguments
///  * 'strike_price_usd' - strike price in usd scaled by 6 decimals (e.g. for $120 -> 120_000_000)
///  * 'spot_price_usd' - spot price in usd scaled by 6 decimals 
/// 
/// #Returns
/// The premium amount in token units (scaled by the asset decimals)
pub fn calculate_premium(
    strike_price_usd: u64,
    spot_price_usd: u64,
    time_to_expity: f64,
    volatility: f64,
    option_type: &OptionType,
    asset_decimals: u8
) -> Result<u64> {
    // Convert to f64 for calculations, adjusting for scale
    let s = spot_price_usd as f64 / 1_000_000.0;
    let k = strike_price_usd as f64 / 1_000_000.0;

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

#[cfg(test)]
mod premiums_tests {
    use super::*;

    #[test]
    fn test_put() {
        let strike_price_usd = 120 * 10u64.pow(6);
        let current_price_usd = 130 * 10u64.pow(6);
        let time_distance= (1 * 24 * 60 * 60) as u64; // 1 day in seconnds
        // let time_distance = 300u64;
        let seconds_per_year: f64 = 365.25 * 24.0 * 60.0 * 60.0;
        let time_to_expire_in_years = time_distance as f64 / seconds_per_year;
        let volatility = 0.8f64;
        let deicmals = 9; //wSOL e.g.


        let premium_put = calculate_premium(
            strike_price_usd, 
            current_price_usd, 
            time_to_expire_in_years, 
            volatility, 
            &OptionType::PUT, 
            deicmals).unwrap();

        assert!(premium_put > 0u64, "Put premium is null");

        let premium_call = calculate_premium(
            strike_price_usd, 
            current_price_usd, 
            time_to_expire_in_years, 
            volatility, 
            &OptionType::CALL, 
            deicmals).unwrap();

        assert!(premium_call > 0u64, "Call premium is null");
    }
}

#[cfg(test)]
mod market_lp_shares_tests {
    use anchor_lang::solana_program::native_token::LAMPORTS_PER_SOL;

    use super::*;

    fn mock_market() -> Market {
        Market {
            id: 1,
            fee_bps: 2,
            lp_minted: 0,
            premiums: 0,
            committed_reserve: 0,
            reserve_supply: 0,
            name: String::from("1 wSOL market"),
            bump: 120,
            volatility_bps: 8000, //80%
            price_feed: String::from("0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d"), 
            asset_decimals: 9,
            asset_mint: Pubkey::new_unique()
        }
    }

    #[test]
    fn calc_lp_shares_issues_correct_token_amount() {
        let mut market = mock_market();

        //First LP deposits 1000 SOL
        let deposit_amount = 1000 * LAMPORTS_PER_SOL; //1_000_000_000
        let lp_1_tokens = calc_lp_shares(deposit_amount, 1, &market).unwrap();

        let lp1_expected_tokens = deposit_amount;
        assert_eq!(lp_1_tokens, lp1_expected_tokens);

        //Update market after deposit
        market.lp_minted = market.lp_minted
            .checked_add(lp_1_tokens).unwrap();
        market.reserve_supply = market.reserve_supply
            .checked_add(deposit_amount).unwrap();

        //Market accumulates 100 SOL worth of premiums
        market.premiums = market.premiums
            .checked_add(100 * LAMPORTS_PER_SOL).unwrap();

        //Second LP deposits 1000 SOL, should get less amount of lp shares
        let lp_2_tokens = calc_lp_shares(deposit_amount, 1, &market).unwrap();

        let lp_2_expected_tokens = 909_090_909_000 as u64;
        assert_eq!(lp_2_tokens, lp_2_expected_tokens);

        assert!(lp_2_tokens < lp_1_tokens, "If there are accumulated premiums, tokens minted to new LPs should be less than the minted amount to previous LPs");
    }

    #[test]
    #[should_panic(expected = "InvalidAmount")]
    fn calc_lp_shares_panics_when_passed_amount_is_zero() {
        let market = mock_market();
        let deposit_amount = 0;
        calc_lp_shares(deposit_amount, 1, &market).unwrap();

    }

    #[test]
    #[should_panic(expected = "DustAmount")]
    fn calc_lp_shares_panics_with_dust_amounts() {
        let mut market = mock_market();

        //First LP deposits 1000 SOL
        let deposit_amount = 1000 * LAMPORTS_PER_SOL; //1_000_000_000
        let lp_1_tokens = calc_lp_shares(deposit_amount, 1, &market).unwrap();

        let lp1_expected_tokens = deposit_amount;
        assert_eq!(lp_1_tokens, lp1_expected_tokens);

        //Update market after deposit
        market.lp_minted = market.lp_minted
            .checked_add(lp_1_tokens).unwrap();
        market.reserve_supply = market.reserve_supply
            .checked_add(deposit_amount).unwrap();

        //Second LP deposits 1000 SOL, should get less amount of lp shares
        calc_lp_shares(1, 1, &market).unwrap();
    }
}