use anchor_lang::prelude::*;

#[error_code]
pub enum CombatSystemError {
    #[msg("Combat has already ended")]
    CombatAlreadyEnded,
    #[msg("Combat has not ended yet")]
    CombatNotEnded,
    #[msg("Unauthorized: signer is not the combat owner")]
    Unauthorized,
    #[msg("Invalid combatant stats provided")]
    InvalidCombatant,
    #[msg("Arithmetic operation overflowed")]
    ArithmeticOverflow,
    #[msg("Combat exceeded maximum turns")]
    MaxTurnsExceeded,
    #[msg("Invalid trigger type for this context")]
    InvalidTrigger,
}
