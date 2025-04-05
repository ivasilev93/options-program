use anchor_lang::prelude::*;

pub const MARKET_SEED: &str = "market";
pub const MARKET_VAULT_SEED: &str = "market_vault";
pub const MARKET_LP_MINT_SEED: &str = "market_lp_mint";

#[account]
#[derive(InitSpace, PartialEq, Eq)]
pub struct Market {

    pub id: u64,

    #[max_len(32)]
    pub name: String,    
    
    pub fee: u64,

    pub bump: u8,

    pub reserve_supply: u64,

    pub premiums: u64,

    pub lp_minted: u64

}