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
    pub option: OptionType,
    pub strike_price_usd: u64, //strike price in usd scaled by 6 decimals 
    pub max_potential_payout_in_tokens: u64,
    pub expiry_stamp: i64,
    pub created_stamp: i64,
    pub quantity: u64
}