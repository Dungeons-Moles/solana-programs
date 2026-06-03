use quasar_lang::prelude::*;

/// Custom error codes for the Player Profile program.
#[error_code]
pub enum PlayerProfileError {
    /// Player profile already exists for this wallet.
    ProfileAlreadyExists,
    /// Display name exceeds 32 character limit.
    NameTooLong,
    /// No available runs remaining.
    NoAvailableRuns,
    /// Signer is not the profile owner.
    Unauthorized,
    /// Arithmetic overflow occurred.
    ArithmeticOverflow,
    /// Active item pool must contain at least 40 items.
    ActivePoolTooSmall,
    /// Item is not unlocked.
    ItemNotUnlocked,
    /// Item index is out of valid range.
    InvalidItemIndex,
    /// Insufficient SOL for purchase.
    InsufficientPayment,
    /// Level is not unlocked.
    LevelNotUnlocked,
    /// Invalid treasury account.
    InvalidTreasury,
    /// Invalid gauntlet pool account.
    InvalidGauntletPool,
    /// Invalid session account.
    InvalidSession,
    /// Session account has invalid owner.
    InvalidSessionOwner,
    /// Session key signer does not match session.
    InvalidSessionSigner,
    /// Direct mutation disabled.
    DirectMutationDisabled,
    /// Invalid session-manager authority.
    InvalidSessionManagerAuthority,
    /// Invalid pit draft queue account.
    InvalidPitDraftQueue,
    /// Cannot update active item pool while queued in pit draft.
    PitDraftQueueLocked,
    /// Invalid skin NFT asset.
    InvalidSkinAsset,
    /// Skin NFT is not owned by the player.
    SkinNotOwned,
    /// Invalid pit draft queue discriminator.
    InvalidPitDraftQueueDiscriminator,
    /// Session account data too short.
    SessionDataTooShort,
    /// Invalid relic asset.
    InvalidRelicAsset,
    /// Relic is not owned by the player.
    RelicNotOwned,
    /// Relic is already registered.
    RelicAlreadyRegistered,
    /// Relic not found.
    RelicNotFound,
    /// Player relic pool is full.
    RelicPoolFull,
}
