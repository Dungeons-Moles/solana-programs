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

    /// Level is not unlocked for this player
    #[msg("Level is not unlocked")]
    LevelNotUnlocked,

    /// Player has no available runs
    #[msg("No available runs remaining")]
    NoAvailableRuns,

    /// Player already has a session on this level
    #[msg("Session already exists for this level")]
    SessionExistsForLevel,

    /// Session can only be ended after death or level completion
    #[msg("Session can only be ended after death or level completion")]
    SessionNotEndable,
}
