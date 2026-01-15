use anchor_lang::prelude::*;

/// Custom error codes for the Player Profile program
#[error_code]
pub enum PlayerProfileError {
    /// Player profile already exists for this wallet
    #[msg("Player profile already exists for this wallet")]
    ProfileAlreadyExists,

    /// Display name exceeds 32 character limit
    #[msg("Display name exceeds 32 character limit")]
    NameTooLong,

    /// Must complete current tier before unlocking next
    #[msg("Must complete current tier before unlocking next")]
    TierNotReached,

    /// Insufficient SOL balance for tier unlock
    #[msg("Insufficient SOL balance for tier unlock")]
    InsufficientFunds,

    /// Signer is not the profile owner
    #[msg("Signer is not the profile owner")]
    Unauthorized,

    /// Level exceeds unlocked tier maximum
    #[msg("Level exceeds unlocked tier maximum")]
    InvalidLevel,

    /// Arithmetic overflow occurred
    #[msg("Arithmetic overflow occurred")]
    ArithmeticOverflow,

    /// Invalid treasury account
    #[msg("Invalid treasury account")]
    InvalidTreasury,
}
