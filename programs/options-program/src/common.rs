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

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug, Copy)]
pub enum SpotDeviation {
    N20,
    N15,
    N10,
    N5,
    P0,
    P5,
    P10,
    P15,
    P20
}

impl SpotDeviation {
    pub fn convert_to_strike(&self, spot_price: u128) -> Result<u128> {
        let deviation = match &self {
            SpotDeviation::N20 => { 80 },
            SpotDeviation::N15 => { 85 },
            SpotDeviation::N10 => { 90 },
            SpotDeviation::N5 => { 95 },
            SpotDeviation::P0 => { 100 },
            SpotDeviation::P5 => { 105 },
            SpotDeviation::P10 => { 110 },
            SpotDeviation::P15 => { 115 },
            SpotDeviation::P20 => { 120 },
        };

        //Imperfect, but good enough to support JUP tokens for demo...
        let step = if spot_price < 100_000_000 {
            1_000_000
        } else {
            100_000_000
        };

        let adjusted = spot_price
                    .checked_mul(deviation).unwrap()
                    .checked_div(100).unwrap();

        let round_up = (adjusted + step / 2) / step;
        Ok((round_up * step) as u128)
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug, Copy)]
pub enum Expiry {
    HOUR1,
    HOUR4,
    DAY1,
    DAY3,
    WEEK
}

#[derive(Copy, Clone, Debug)]
pub enum ExpiryError {
    InvalidExpirySetting,
}

impl Expiry {
    pub fn to_seconds(&self) -> std::result::Result<u64, ExpiryError> {
        match self {
            Expiry::HOUR1 => Ok(60 * 60),
            Expiry::HOUR4 => Ok(4 * 60 * 60),
            Expiry::DAY1 => Ok(24 * 60 * 60),
            Expiry::DAY3 => Ok(3 * 24 * 60 * 60),
            Expiry::WEEK => Ok(7 * 24 * 60 * 60),
        }
    }
}

impl TryFrom<u8> for Expiry {
    type Error = ExpiryError;

    fn try_from(value: u8) -> std::result::Result<Expiry, ExpiryError> {
        match value {
            0 => Ok(Expiry::HOUR1),
            1 => Ok(Expiry::HOUR4),
            2 => Ok(Expiry::DAY1),
            3 => Ok(Expiry::DAY3),
            4 => Ok(Expiry::WEEK),
            _ => Err(ExpiryError::InvalidExpirySetting)
        }
    }   
}

pub fn calc_time_distance(stamp_now: i64, expiry_stamp: i64) -> Result<f64> {
    let time_distance = expiry_stamp - stamp_now;

    let seconds_in_day: i64 = 86400;
    require!(time_distance > 0, CustomError::InvalidExpiry);
    require!(time_distance / seconds_in_day <= 30, CustomError::InvalidExpiry);

    let seconds_per_year: f64 = 365.25 * 24.0 * 60.0 * 60.0;
    let time_to_expire_in_years = time_distance as f64 / seconds_per_year;

    Ok(time_to_expire_in_years)
}