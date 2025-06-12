use crate::state::market::*;
use crate::common::*;
use anchor_lang::{prelude::Pubkey, solana_program::native_token::LAMPORTS_PER_SOL};

#[cfg(test)]
mod market_issue_lp_shares_tests {
    use crate::math::lp_shares::*;

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

    // #[test]
    // #[should_panic(expected = "InvalidAmount")]
    // fn calc_lp_shares_panics_when_passed_amount_is_zero() {
    //     let market = mock_market();
    //     let deposit_amount = 0;
    //     calc_lp_shares(deposit_amount, 1, &market).unwrap();
    // }

    //     calc_lp_shares(1, 1, &market).unwrap();
    // }
}



#[cfg(test)]
mod premium_display {
    use crate::math::{lp_shares::calc_withdraw_amount_from_lp_shares, premium::*};

    use super::*;

    fn mock_market() -> Market {
        Market {
            id: 1,
            fee_bps: 5,
            lp_minted: 0,
            premiums: 0,
            committed_reserve: 0,
            reserve_supply: u64::MAX,
            name: String::from("1 wSOL market"),
            bump: 120,
            hour1_volatility_bps: 8000, //80%,
            hour4_volatility_bps: 7000, //70%,
            day1_volatility_bps: 6000, 
            day3_volatility_bps: 7000, 
            week_volatility_bps: 7000, 
            vol_last_updated: 0,
            price_feed: String::from("0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d"), 
            asset_decimals: 9,
            asset_mint: Pubkey::new_unique()
        }
    }

    #[test]
    fn calculate_premiums() {
        // Test cases: (strike_price, spot_price)
        // Prices are scaled by 10^8 (e.g., 11,000,000,000 = $110)
        let test_cases = vec![
            (11_000_000_000, 12_000_000_000), // Strike < Spot, 
            (11_000_000_000, 14_000_000_000), 
            (11_000_000_000, 16_000_000_000), 
            (11_000_000_000, 17_000_000_000), 
            (11_000_000_000, 10_000_000_000), //Strike > Spot
            (10_000_000_000, 8_000_000_000),  
            (10_000_000_000, 6_000_000_000),  
            (10_000_000_000, 9_000_000_000),  
            (10_000_000_000, 3_000_000_000),  
        ];

        const SCALE: u128 = 100_000_000; // 10^8
        let market = &mock_market();

        for (strike, spot) in test_cases {            
           let (prem_h1,fee1,_) = calculate_option_premium(
                strike, 
                spot,
                Expiry::HOUR1,
                market,
                &OptionType::CALL,
                1
            ).unwrap();

            let (prem_h4,fee2,_) = calculate_option_premium(
                strike, 
                spot,
                Expiry::HOUR4,
                market,
                &OptionType::CALL,
                1
            ).unwrap();

            let (prem_d,fee3,_) = calculate_option_premium(
                strike, 
                spot,
                Expiry::DAY1,
                market,
                &OptionType::CALL,
                1
            ).unwrap();

            let (prem_3d,fee4,_) = calculate_option_premium(
                strike, 
                spot,
                Expiry::DAY3,
                market,
                &OptionType::CALL,
                1
            ).unwrap();

            let (prem_7d, fee5,_) = calculate_option_premium(
                strike, 
                spot,
                Expiry::DAY3,
                market,
                &OptionType::CALL,
                1
            ).unwrap();

            let (x1, _) = calculate_collateral(strike, spot, &OptionType::CALL, market, Expiry::HOUR1, 1).unwrap();
            let (x2, _) = calculate_collateral(strike, spot, &OptionType::CALL, market, Expiry::HOUR4, 1).unwrap();
            let (x3, _) = calculate_collateral(strike, spot, &OptionType::CALL, market, Expiry::DAY1, 1).unwrap();
            let (x4, _) = calculate_collateral(strike, spot, &OptionType::CALL, market, Expiry::DAY3, 1).unwrap();
            let (x5, _) = calculate_collateral(strike, spot, &OptionType::CALL, market, Expiry::WEEK, 1).unwrap();

           
            // Print for debugging
            println!(
                "Premiums usd Strike: {}, Spot: {}, H: {}, 4H: {}, D: {}, 3D: {}, 7D: {}",
                strike, spot,
                prem_h1 as f64 / SCALE as f64,
                prem_h4 as f64 / SCALE as f64,
                prem_d as f64 / SCALE as f64,
                prem_3d as f64 / SCALE as f64,
                prem_7d as f64 / SCALE as f64,
            );

            println!(
                "Premiums tokens Strike: {}, Spot: {}, H: {}, 4H: {}, D: {}, 3D: {}, 7D: {}",
                strike, spot,
                fee1 as f64 / 10_u64.pow(market.asset_decimals as u32) as f64,
                fee2 as f64 / 10_u64.pow(market.asset_decimals as u32) as f64,
                fee3 as f64 / 10_u64.pow(market.asset_decimals as u32) as f64,
                fee4 as f64 / 10_u64.pow(market.asset_decimals as u32) as f64,
                fee5 as f64 / 10_u64.pow(market.asset_decimals as u32) as f64,
            );

            println!(
                "Collateral H: {}, COL 4H: {}, COL D: {}, COL 3D: {}, COL 7D: {}",
                x1 as f64 / SCALE as f64,
                x2 as f64 / SCALE as f64,
                x3 as f64 / SCALE as f64,
                x4 as f64 / SCALE as f64,
                x5 as f64 / SCALE as f64,
            );

            println!("");
        }
    }

    #[test]
    fn check_lp_shares_calc() {
        let mut market = mock_market();
        market.reserve_supply = 2_000;
        market.lp_minted = 1_800_000;
        market.committed_reserve = 1500;
        market.premiums = 200;

        let test_cases = vec![1000, 10_000, 100_000, 500_000, 1_000_000];
        for c in &test_cases {
            let (x, y) = calc_withdraw_amount_from_lp_shares(*c, &market).unwrap();

            print!("LP {} -> {} (capped - {})", c, x, y);
        }

        market.committed_reserve = 0;
        for c in test_cases {
            let (x, y) = calc_withdraw_amount_from_lp_shares(c, &market).unwrap();

            print!("LP {} -> {} (capped - {})", c, x, y);
        }


    }
}

