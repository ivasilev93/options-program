use anchor_lang::prelude::*;

pub const USR_ACC_SEED: &str = "account";

#[account]
#[derive(InitSpace, PartialEq, Eq)]
pub struct UserAccount {
    pub balance: u64,
    pub options: [OptionOrder; 32]
}

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, PartialEq, Eq)]
pub enum OptionType {
    PUT,
    CALL
}

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, PartialEq, Eq)]
pub struct OptionOrder {
    pub option_type: OptionType,
    pub strike_price: u64,
    pub expiry: i64,
    pub premium: u64
}