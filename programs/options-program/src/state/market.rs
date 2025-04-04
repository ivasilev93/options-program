use anchor_lang::prelude::*;

pub const MARKET_SEED: &str = "market";

#[account]
#[derive(InitSpace, PartialEq, Eq)]
pub struct Market {
    id: u8,
    #[max_len(32)]
    name: String,
    collateral: u64,
    fee: u64,
    bump: u8

}