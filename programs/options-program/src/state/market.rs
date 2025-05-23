use anchor_lang::prelude::*;
use crate::{common::*};

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
    #[max_len(70)]
    pub price_feed: String,       // Pyth feed (TOKEN)/USD
    pub asset_decimals: u8,
    pub hour1_volatility_bps: u32,      // 1bps = 0.01%
    pub hour4_volatility_bps: u32,  
    pub day1_volatility_bps: u32,  
    pub day3_volatility_bps: u32,  
    pub week_volatility_bps: u32,
    pub vol_last_updated: i64  
}

impl Market {
    pub fn get_volatility(&self, expiry_setting: &Expiry) -> Result<u32> {
        //Expiry to be measured as distance in seconds
        match expiry_setting {
            Expiry::HOUR1 => {
                Ok(self.hour1_volatility_bps)
                // let distasnce = 60 * 60;
                // (self.hour1_volatility_bps as f64 / BASIS_POINTS_DENOMINATOR as f64, stamp_now + distasnce)
            },
            Expiry::HOUR4 => {
                Ok(self.hour4_volatility_bps)
                // let distasnce = 4 * 60 * 60;
                // (self.hour4_volatility_bps as f64 / BASIS_POINTS_DENOMINATOR as f64, stamp_now + distasnce)
            },
            Expiry::DAY1 => {
                Ok(self.day1_volatility_bps)
                // let distasnce = 24 * 60 * 60;
                // (self.day1_volatility_bps as f64 / BASIS_POINTS_DENOMINATOR as f64, stamp_now + distasnce)
            },
            Expiry::DAY3 => {
                Ok(self.day3_volatility_bps)
                // let distasnce = 3 * 24 * 60 * 60;
                // (self.day3_volatility_bps as f64 / BASIS_POINTS_DENOMINATOR as f64, stamp_now + distasnce)
            },
            Expiry::WEEK => {
                Ok(self.week_volatility_bps)
                // let distasnce = 7 * 24 * 60 * 60;
                // (self.week_volatility_bps as f64 / BASIS_POINTS_DENOMINATOR as f64, stamp_now + distasnce)
            },
        }
    }

    
}
