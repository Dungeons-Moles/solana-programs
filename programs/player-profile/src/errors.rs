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

    /// Active item pool has fewer than 40 items
    #[msg("Active item pool must contain at least 40 items")]
    ActivePoolTooSmall,

    /// Item is not unlocked in player's unlocked_items bitmask
    #[msg("Item is not unlocked")]
    ItemNotUnlocked,

    /// Item index is out of valid range (0-79)
    #[msg("Item index is out of valid range")]
    InvalidItemIndex,

    /// Insufficient SOL balance for purchase
    #[msg("Insufficient SOL for purchase")]
    InsufficientPayment,

    /// Attempted to access a level that is not yet unlocked
    #[msg("Level is not unlocked")]
    LevelNotUnlocked,

    /// Treasury account does not match expected address
    #[msg("Invalid treasury account")]
    InvalidTreasury,
}
