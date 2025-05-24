use anchor_lang::prelude::*;
use crate::common::OptionType;

pub const USR_ACC_SEED: &str = "account";

#[account(zero_copy)]
#[derive(InitSpace, PartialEq, Eq)]
pub struct UserAccount {
    pub options: [OptionOrder; 32]
}

#[derive(PartialEq, Eq,InitSpace)]
#[zero_copy]
#[repr(C)]
pub struct OptionOrder {
    pub strike_price: u64,  //scaled by 10^8
    pub expiry: i64,
    pub premium: u64,
    pub premium_in_usd: u64,
    pub quantity: u64,
    pub max_potential_payout_in_tokens: u64,
    pub market_ix: u16,
    pub option_type: u8,
    pub ix: u8,
    pub is_used: u8,
    pub padding: [u8; 3]
}

impl OptionOrder {
    pub fn is_initialized(&self) -> bool {
        self.is_used == 1        
    }

    pub fn clear(&mut self) {
        self.expiry = 0;
        self.market_ix = 0;
        self.option_type = u8::from(OptionType::PUT);
        self.premium = 0;
        self.strike_price = 0;
        self.quantity = 0;
        self.max_potential_payout_in_tokens = 0;
        self.ix = 0;
        self.is_used = 0;
    }
}

impl UserAccount {
    pub fn get_available_slot(&self) -> Option<usize> {
        self.options.iter()
        .position(|o| !o.is_initialized())
    }
}