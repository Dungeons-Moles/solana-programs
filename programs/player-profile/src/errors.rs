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

    /// No available runs remaining
    #[msg("No available runs remaining")]
    NoAvailableRuns,

    /// Signer is not the profile owner
    #[msg("Signer is not the profile owner")]
    Unauthorized,

    /// Arithmetic overflow occurred
    #[msg("Arithmetic overflow occurred")]
    ArithmeticOverflow,
}
