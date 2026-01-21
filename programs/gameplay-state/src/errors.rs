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

    #[msg("Unauthorized: only session owner can modify state")]
    Unauthorized,

    #[msg("Session is not active")]
    SessionNotActive,

    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,
}
