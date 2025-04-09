use anchor_lang::prelude::*;

#[error_code]
pub enum CustomError {
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("SlippageExceeded")]
    SlippageExceeded,
    #[msg("InvalidAmount")]
    InvalidAmount,
    #[msg("DustAmount")]
    DustAmount,
    #[msg("Overflow")]
    Overflow,
    #[msg("OrdersLimitExceeded")]
    OrdersLimitExceeded,
    #[msg("InvalidExpiry")]
    InvalidExpiry,
    #[msg("InsufficientColateral")]
    InsufficientColateral,
}