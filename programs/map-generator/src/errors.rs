use anchor_lang::prelude::*;

/// Custom error codes for the Map Generator program
#[error_code]
pub enum MapGeneratorError {
    /// Campaign level must be 0-80
    #[msg("Campaign level must be 0-80")]
    InvalidLevel,

    /// Signer is not the config admin
    #[msg("Signer is not the config admin")]
    Unauthorized,

    /// Map config is already initialized
    #[msg("Map config is already initialized")]
    ConfigAlreadyInitialized,

    /// Map hash does not match expected value
    #[msg("Map hash does not match expected value")]
    InvalidMapHash,

    /// Batch update exceeds maximum allowed
    #[msg("Batch update exceeds maximum allowed")]
    TooManyUpdates,
}
