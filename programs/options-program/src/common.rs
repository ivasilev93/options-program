use anchor_lang::prelude::*;

use crate::errors::CustomError;

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

pub fn calc_time_distance(clock: &Clock, expiry_stamp: i64) -> Result<f64> {
    let stamp_now = clock.unix_timestamp;
    let time_distance = expiry_stamp - stamp_now;

    let seconds_in_day: i64 = 86400;
    require!(time_distance > 0, CustomError::InvalidExpiry);
    require!(time_distance / seconds_in_day <= 30, CustomError::InvalidExpiry);

    let seconds_per_year: f64 = 365.25 * 24.0 * 60.0 * 60.0;
    let time_to_expire_in_years = time_distance as f64 / seconds_per_year;

    Ok(time_to_expire_in_years)
}