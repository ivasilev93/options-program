use anchor_lang::prelude::*;
use crate::common::OptionType;

#[event]
pub struct MakerDepositEvent {
    pub user: Pubkey,
    pub market: u16,
    pub market_name: String,
    pub market_asset_mint: Pubkey,
    pub market_reserve_before: u64,
    pub market_reserve_after: u64,
    pub tokens_deposited: u64,
    pub lp_tokens_minted: u64,
}

#[event]
pub struct OptionBought {
    pub user: Pubkey,
    pub market: u16,
    pub option_ix: u8,
    pub option: OptionType,
    pub strike_price_usd: u64, //strike price in usd scaled by 6 decimals 
    pub bought_at_price_usd: u64, //bought price in usd scaled by 6 decimals 
    pub max_potential_payout_in_tokens: u64,
    pub expiry_stamp: i64,
    // pub created_stamp: i64,
    pub quantity: u64
}

#[event]
pub struct OptionExercised {
    pub user: Pubkey,
    pub market: u16,
    pub option_ix: u8,
    pub option: OptionType,
    pub timestamp: i64,
    pub quantity: u64,
    pub profit_usd: u64, 
    pub user_payout: u64, 
}

#[event]
pub struct MakerWithdrawEvent {
    pub user: Pubkey,
    pub market: u16,
    pub market_name: String,
    pub market_asset_mint: Pubkey,
    pub reserve_before: u64,
    pub reserve_after: u64,
    pub premiums_before: u64,
    pub premiums_after: u64,
    pub lp_tokens_before: u64,
    pub lp_tokens_after: u64,
    pub tokens_withdrawn: u64,
}