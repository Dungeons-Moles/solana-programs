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

    /// Gauntlet pool account does not match expected address
    #[msg("Invalid gauntlet pool account")]
    InvalidGauntletPool,

    /// Invalid session account for CPI authorization
    #[msg("Invalid session account")]
    InvalidSession,

    /// Session account is not owned by the session-manager program
    #[msg("Session account has invalid owner")]
    InvalidSessionOwner,

    /// Session key signer does not match the session's stored session key signer
    #[msg("Session key signer does not match session")]
    InvalidSessionSigner,

    /// Direct profile mutations are disabled; use authorized CPI path.
    #[msg("Direct mutation disabled")]
    DirectMutationDisabled,

    /// Session-manager authority PDA is invalid.
    #[msg("Invalid session-manager authority")]
    InvalidSessionManagerAuthority,

    /// Pit draft queue account is invalid or malformed
    #[msg("Invalid pit draft queue account")]
    InvalidPitDraftQueue,

    /// Active item pool cannot be changed while queued in pit draft
    #[msg("Cannot update active item pool while queued in pit draft")]
    PitDraftQueueLocked,

    /// Skin NFT asset is invalid (not a Metaplex Core AssetV1)
    #[msg("Invalid skin NFT asset")]
    InvalidSkinAsset,

    /// Skin NFT is not owned by the player
    #[msg("Skin NFT is not owned by the player")]
    SkinNotOwned,
}
