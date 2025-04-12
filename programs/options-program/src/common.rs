use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, PartialEq, Eq, Debug)]
pub enum OptionType {
    PUT,
    CALL
}

