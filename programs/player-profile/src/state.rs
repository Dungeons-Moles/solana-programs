use anchor_lang::prelude::*;

/// Player profile account storing identity and progression data.
/// PDA Seeds: [b"player", owner.key()]
#[account]
#[derive(InitSpace)]
pub struct PlayerProfile {
    /// Wallet address that owns this profile
    pub owner: Pubkey,

    /// Display name (max 32 chars)
    #[max_len(32)]
    pub name: String,

    /// Total dungeon runs completed
    pub total_runs: u32,

    /// Current campaign level (0-80+)
    pub current_level: u8,

    /// Remaining available dungeon runs
    pub available_runs: u32,

    /// Unix timestamp of profile creation
    pub created_at: i64,

    /// PDA bump seed
    pub bump: u8,
}

impl PlayerProfile {
    /// Seed prefix for PDA derivation
    pub const SEED_PREFIX: &'static [u8] = b"player";
}
