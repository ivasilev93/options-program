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
    #[msg("Underflow")]
    Underflow,
    #[msg("OrdersLimitExceeded")]
    OrdersLimitExceeded,
    #[msg("InvalidExpiry")]
    InvalidExpiry,
    #[msg("InsufficientColateral")]
    InsufficientColateral,
    #[msg("InvalidPriceFeed")]
    InvalidPriceFeed,
    #[msg("ExerciseIsOverdue")]
    ExerciseIsOverdue,
    #[msg("InsufficientShares")]
    InsufficientShares,
    #[msg("InvalidState")]
    InvalidState,
    #[msg("PremiumCalcError")]
    PremiumCalcError,
    #[msg("InvalidPrices")]
    InvalidPrices,
    #[msg("Cannot withdraw. Funds are collateral to active options")]
    CannotWithdraw,
}