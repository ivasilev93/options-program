use anchor_lang::prelude::*;
use crate::common::OptionType;

pub const USR_ACC_SEED: &str = "account";

#[account]
#[derive(InitSpace, PartialEq, Eq)]
pub struct UserAccount {
    // pub balance: u64,
    pub options: [OptionOrder; 32]
}

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, PartialEq, Eq)]
pub struct OptionOrder {
    pub market_ix: u16,
    pub option_type: OptionType,
    pub strike_price: u64,
    pub expiry: i64,
    pub premium: u64
}

impl OptionOrder {
    pub fn is_initialized(&self) -> bool {
        self.expiry != 0 && self.premium != 0 && self.strike_price != 0
    }
}

impl UserAccount {
    pub fn get_available_slot(&self, market_ix: u16) -> Option<usize> {
        self.options.iter()
        .position(|o| !o.is_initialized())
    }
}