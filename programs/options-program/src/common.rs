use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug, Copy)]
pub enum OptionType {
    PUT,
    CALL
}

#[derive(Copy, Clone, Debug)]
pub enum OptionTypeError {
    InvalidValue,
}

impl From<OptionType> for u8 {
    fn from(value: OptionType) -> Self {
        match value {
            OptionType::PUT => 0,
            OptionType::CALL => 1,
        }
    }
}

impl TryFrom<u8> for OptionType {
    type Error = OptionTypeError;

    fn try_from(value: u8) -> std::result::Result<OptionType, OptionTypeError> {
        match value {
            0 => Ok(OptionType::PUT),
            1 => Ok(OptionType::CALL),
            _ => Err(OptionTypeError::InvalidValue)
        }
    }
}

