use anchor_lang::prelude::*;

#[error_code]
pub enum BossSystemError {
    #[msg("Stage must be between 1 and 80")]
    InvalidStage,
    #[msg("No boss found for the given parameters")]
    BossNotFound,
}
