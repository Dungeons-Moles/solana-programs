use anchor_lang::prelude::*;

#[error_code]
pub enum FieldEnemiesError {
    #[msg("Invalid act number, must be 1-4")]
    InvalidAct,

    #[msg("Session is not in active state")]
    SessionNotActive,

    #[msg("MapEnemies account already initialized")]
    MapEnemiesAlreadyExists,

    #[msg("No enemy found at specified position")]
    EnemyNotFound,

    #[msg("Enemy at position already defeated")]
    EnemyAlreadyDefeated,

    #[msg("Caller not authorized")]
    UnauthorizedCaller,

    #[msg("Invalid archetype ID, must be 0-11")]
    InvalidArchetypeId,

    #[msg("Invalid tier value, must be 0-2")]
    InvalidTier,

    #[msg("Position out of map bounds")]
    PositionOutOfBounds,

    #[msg("Maximum enemy count exceeded")]
    MaxEnemiesExceeded,
}
