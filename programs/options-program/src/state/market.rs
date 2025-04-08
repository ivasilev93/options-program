use anchor_lang::{prelude::*, solana_program::native_token::LAMPORTS_PER_SOL};

use crate::errors::CustomError;

pub const MARKET_SEED: &str = "market";
pub const MARKET_VAULT_SEED: &str = "market_vault";
pub const MARKET_LP_MINT_SEED: &str = "market_lp_mint";

#[account]
#[derive(InitSpace, PartialEq, Eq)]
pub struct Market {

    pub id: u16,
    #[max_len(32)]
    pub name: String,        
    pub fee: u64,
    pub bump: u8,
    pub reserve_supply: u64,
    pub premiums: u64,
    pub lp_minted: u64
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

#[cfg(test)]
mod market_tests {
    use anchor_lang::solana_program::native_token::LAMPORTS_PER_SOL;

    use super::*;

    fn mock_market() -> Market {
        Market {
            id: 1,
            fee: 2,
            lp_minted: 0,
            premiums: 0,
            reserve_supply: 0,
            name: String::from("1 wSOL market"),
            bump: 120
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