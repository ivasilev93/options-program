use anchor_lang::prelude::*;

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