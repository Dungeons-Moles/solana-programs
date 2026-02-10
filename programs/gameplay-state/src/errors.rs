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
}
