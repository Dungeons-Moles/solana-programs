use anchor_lang::prelude::*;

#[error_code]
pub enum GameplayStateError {
    #[msg("Target position is out of map boundaries")]
    OutOfBounds,

    #[msg("Not enough moves remaining for this action")]
    InsufficientMoves,

    #[msg("Target position is not adjacent to current position")]
    NotAdjacent,

    #[msg("Stat value would overflow")]
    StatOverflow,

    #[msg("HP cannot go below 0")]
    HpUnderflow,

    #[msg("Gold cannot go below 0")]
    GoldUnderflow,

    #[msg("Invalid stat modification")]
    InvalidStatModification,

    #[msg("Boss fight already triggered")]
    BossFightAlreadyTriggered,

    #[msg("Boss fight not ready - must exhaust moves in Night3 phase first")]
    BossFightNotReady,

    #[msg("Unauthorized: only session owner can modify state")]
    Unauthorized,

    #[msg("Session is not active")]
    SessionNotActive,

    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,

    #[msg("No enemy at the specified position")]
    EnemyNotAtPosition,

    #[msg("Player has been defeated")]
    PlayerDefeated,

    #[msg("Player is dead - no further actions allowed")]
    PlayerDead,

    #[msg("Run is already completed - no further actions allowed")]
    RunCompleted,

    #[msg("Invalid week value")]
    InvalidWeek,

    #[msg("Invalid enemy tier")]
    InvalidEnemyTier,
    #[msg("Invalid session account")]
    InvalidSession,
    #[msg("Invalid session owner program")]
    InvalidSessionOwner,

    #[msg("Skip to day can only be used during night phases")]
    NotNightPhase,

    #[msg("Invalid HP bonus value")]
    InvalidHpBonus,

    #[msg("Test-only instruction is disabled in production builds")]
    TestOnlyInstructionDisabled,

    #[msg("Player is already queued in pit draft")]
    PitDraftAlreadyQueued,

    #[msg("Pit draft waiting player data is missing or inconsistent")]
    PitDraftInvalidWaitingState,

    #[msg("Waiting player account inputs are missing")]
    PitDraftMissingWaitingAccounts,

    #[msg("Cannot match against yourself in pit draft")]
    PitDraftSelfMatch,

    #[msg("Provided waiting accounts do not match queued player")]
    PitDraftWaitingAccountMismatch,

    #[msg("Invalid pit draft fee account")]
    InvalidPitDraftFeeAccount,

    #[msg("Pit draft vault has insufficient funds")]
    PitDraftInsufficientVaultFunds,

    #[msg("Active pool does not contain enough items for pit draft loadout")]
    PitDraftInsufficientPoolItems,

    #[msg("Player is already queued in duels for this seed")]
    DuelAlreadyQueued,

    #[msg("Duel queue is already full for this seed")]
    DuelQueueFull,

    #[msg("Duel run is not finished yet")]
    DuelRunNotFinished,

    #[msg("Provided seed does not match session generated map seed")]
    DuelSeedMismatch,

    #[msg("Session is not configured for duel mode")]
    DuelInvalidRunMode,

    #[msg("Player is not queued in this duel")]
    DuelNotQueued,

    #[msg("Provided game state does not match queued duel participant")]
    DuelGameStateMismatch,

    #[msg("Cannot match against yourself in duels")]
    DuelSelfMatch,

    #[msg("Invalid duel fee account")]
    InvalidDuelFeeAccount,

    #[msg("Required duel wallet account is missing")]
    DuelMissingWalletAccount,

    #[msg("Duel queue state is invalid or inconsistent")]
    DuelInvalidQueueState,

    #[msg("Gauntlet mode is not initialized")]
    GauntletNotInitialized,

    #[msg("Invalid gauntlet fee account")]
    InvalidGauntletFeeAccount,

    #[msg("Invalid gauntlet week")]
    InvalidGauntletWeek,

    #[msg("Gauntlet run is not active")]
    GauntletRunNotActive,

    #[msg("Gauntlet run already ended")]
    GauntletRunEnded,

    #[msg("Gauntlet rewards already claimed for this epoch")]
    GauntletAlreadyClaimed,

    #[msg("Gauntlet score account mismatch")]
    GauntletScoreMismatch,

    #[msg("Gauntlet epoch pool is not finalized")]
    GauntletEpochNotFinalized,

    #[msg("Gauntlet entry already paid for this run")]
    GauntletAlreadyEntered,

    #[msg("Run mode can only be configured at session start")]
    RunModeConfigurationLocked,

    #[msg("Invalid max weeks value for run mode")]
    InvalidRunModeMaxWeeks,

    #[msg("Gauntlet session already settled")]
    GauntletAlreadySettled,

    #[msg("Campaign level must be between 1 and 40")]
    InvalidCampaignLevel,

    #[msg("Account still has data — cannot close as empty")]
    AccountNotEmpty,

    #[msg("VRF state has not been fulfilled yet")]
    VrfNotFulfilled,

    #[msg("VRF state has not been requested yet")]
    VrfNotRequested,
}
