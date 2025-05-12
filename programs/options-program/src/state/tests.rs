use crate::state::market::*;
use crate::common::*;
use anchor_lang::{prelude::Pubkey, solana_program::native_token::LAMPORTS_PER_SOL};

#[cfg(test)]
mod premiums_tests {
    use super::*;

    #[test]
    fn premium_calls() {
        println!();

        let volatility = 0.8f64;
        let deicmals = 9; //wSOL e.g.

        //Strike price, current price, time distance in days, 
        let test_cases = vec![
            (135.0, 133.0, 1),
            (140.0, 133.0, 1),
            (145.0, 133.0, 1),
            (135.0, 133.0, 7),
            (140.0, 133.0, 7),
            (145.0, 133.0, 7),
            (135.0, 133.0, 30),
            (140.0, 133.0, 30),
            (145.0, 133.0, 30)
        ];

        for test_case in test_cases {
            let (strike_price, curr_price, days ) = test_case;

            let time_distance= (days * 24 * 60 * 60) as u64; // days in seconnds
            let seconds_per_year: f64 = 365.25 * 24.0 * 60.0 * 60.0;
            let time_to_expire_in_years = time_distance as f64 / seconds_per_year;           

            let premium_call = calculate_premium(
                strike_price, 
                curr_price, 
                time_to_expire_in_years, 
                volatility, 
                &OptionType::CALL, 
                deicmals).unwrap();

            let premium_usd = (premium_call as f64 / 1_000_000_000f64) * curr_price as f64;
    
            println!("Strike: ${strike_price}, Current: ${curr_price}, Interval: {} days, USD premium: ${:.2}, Tokens premium: {}", 
            days, premium_usd, premium_call);
            assert!(premium_call > 0u64, "Call premium is null");
        }
    }
}

#[cfg(test)]
mod market_issue_lp_shares_tests {
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
            hour1_volatility_bps: 10000, //1%,
            hour4_volatility_bps: 10000, //1%,
            day1_volatility_bps: 10000, //1%,
            day3_volatility_bps: 10000, //1%,
            week_volatility_bps: 10000, //1%,
            vol_last_updated: 0,
            price_feed: String::from("0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d"), 
            asset_decimals: 9,
            asset_mint: Pubkey::new_unique()
        }
    }

    #[test]
    fn calc_lp_shares_issues_correct_token_amount() {
        let mut market = mock_market();

        println!("Testing for two LPs with same deposit amount - 1000 SOL / 1_000_000_000_000...");
        let deposit_amount = 1000 * LAMPORTS_PER_SOL; 
        
        //Alice deposits 1000 SOL
        let alice_lp_tokens = calc_lp_shares(deposit_amount, 1, &market).unwrap();
        let alice_expected_lp_tokens = deposit_amount * 1000;
        assert_eq!(alice_lp_tokens, alice_expected_lp_tokens);
        println!("Alice deposits: asset_tokens: {}, lp minted: {}. 1 to 1000 ratio", deposit_amount, alice_lp_tokens);
        
        //Update market after deposit
        market.lp_minted = market.lp_minted
            .checked_add(alice_lp_tokens).unwrap();
        market.reserve_supply = market.reserve_supply
            .checked_add(deposit_amount).unwrap();

        //Market accumulates 100 SOL worth of premiums
        println!("Market accrues 100 SOL premiums...");
        market.premiums = market.premiums
            .checked_add(100 * LAMPORTS_PER_SOL).unwrap();

        println!("Market state: premiums: {}, reserve: {}, lp: {}", market.premiums, market.reserve_supply, market.lp_minted);

        //Bob deposits 1000 SOL, should get less amount of lp shares
        let bob_lp_tokens = calc_lp_shares(deposit_amount, 1, &market).unwrap();
        println!("Bob deposits: asset_tokens: {}, lp minted: {}", deposit_amount, bob_lp_tokens);

        let bob_expected_lp_tokens = 909_090_909_000_000 as u64;
        assert_eq!(bob_lp_tokens, bob_expected_lp_tokens);

        //If there are accumulated premiums, tokens minted to new LPs should be less than the minted amount to previous LPs
        assert!(bob_lp_tokens < alice_expected_lp_tokens, "Incorrect lp token amount for Bob");

        //Update market after 2nd deposit
        market.lp_minted = market.lp_minted
            .checked_add(bob_lp_tokens).unwrap();
        market.reserve_supply = market.reserve_supply
          .checked_add(deposit_amount).unwrap();
        
        println!("Market state: premiums: {}, reserve: {}, lp: {}", market.premiums, market.reserve_supply, market.lp_minted);

        //Alice looks to withdraw
        let (alice_received_asset_tokens, burned_shares) = calc_withdraw_amount_from_lp_shares(alice_lp_tokens, &market).unwrap();
        println!("Alice burns lp: {}, asset_token share: {}, burned lp: {}", alice_lp_tokens, alice_received_asset_tokens, burned_shares);
        assert!(alice_received_asset_tokens > deposit_amount, "Received asset tokens should be more then the deposited amount");        

        //Market accumulates another 100 SOL worth of premiums
        println!("Market accrues another 100 SOL premiums...");
        market.premiums = market.premiums
            .checked_add(100 * LAMPORTS_PER_SOL).unwrap();

        println!("Market state: premiums: {}, reserve: {}, lp: {}", market.premiums, market.reserve_supply, market.lp_minted);

        let (alice_received_asset_tokens, burned_shares) = calc_withdraw_amount_from_lp_shares(alice_lp_tokens, &market).unwrap();
        println!("Alice burns lp: {}, asset_token share: {}, burned lp: {}", alice_lp_tokens, alice_received_asset_tokens, burned_shares);
        assert!(alice_received_asset_tokens > deposit_amount, "Alice Incorrect withdraw amount"); 

        let (bob_received_asset_tokens, burned_shares) = calc_withdraw_amount_from_lp_shares(bob_expected_lp_tokens, &market).unwrap();
        println!("Bob burns lp: {}, asset_token share: {}, burned lp: {}", bob_expected_lp_tokens, bob_received_asset_tokens, burned_shares);
        assert!(bob_received_asset_tokens > deposit_amount, "Bob Incorrect withdraw amount"); 
        println!("Total received asset share: {}", alice_received_asset_tokens + bob_received_asset_tokens);
        
    }

    #[test]
    #[should_panic(expected = "InvalidAmount")]
    fn calc_lp_shares_panics_when_passed_amount_is_zero() {
        let market = mock_market();
        let deposit_amount = 0;
        calc_lp_shares(deposit_amount, 1, &market).unwrap();
    }

    // #[test]
    // #[should_panic(expected = "DustAmount")]
    // fn calc_lp_shares_panics_with_dust_amounts() {
    //     let mut market = mock_market();

    //     //First LP deposits 1000 SOL
    //     let deposit_amount = 1000 * LAMPORTS_PER_SOL; //1_000_000_000
    //     let lp_1_tokens = calc_lp_shares(deposit_amount, 1, &market).unwrap();

    //     let lp1_expected_tokens = deposit_amount;
    //     assert_eq!(lp_1_tokens, lp1_expected_tokens);

    //     //Update market after deposit
    //     market.lp_minted = market.lp_minted
    //         .checked_add(lp_1_tokens).unwrap();
    //     market.reserve_supply = market.reserve_supply
    //         .checked_add(deposit_amount).unwrap();

    //     //Second LP deposits 1000 SOL, should get less amount of lp shares
    //     calc_lp_shares(1, 1, &market).unwrap();
    // }
}

