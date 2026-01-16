use anchor_lang::prelude::*;

/// Custom error codes for the Session Manager program
#[error_code]
pub enum SessionManagerError {
    /// Player already has an active session
    #[msg("Player already has an active session")]
    SessionAlreadyActive,

    /// No active session found for player
    #[msg("No active session found for player")]
    SessionNotFound,

    /// Session must be delegated first
    #[msg("Session must be delegated first")]
    SessionNotDelegated,

    /// Session is already delegated
    #[msg("Session is already delegated")]
    SessionAlreadyDelegated,

    /// Session has not timed out yet
    #[msg("Session has not timed out yet")]
    SessionNotTimedOut,

    /// Signer is not the session owner
    #[msg("Signer is not the session owner")]
    Unauthorized,

    /// Campaign level exceeds player's unlocked tier
    #[msg("Campaign level exceeds player's unlocked tier")]
    InvalidCampaignLevel,

    /// Player profile does not exist
    #[msg("Player profile does not exist")]
    ProfileNotFound,

    /// Arithmetic overflow occurred
    #[msg("Arithmetic overflow occurred")]
    ArithmeticOverflow,
}
