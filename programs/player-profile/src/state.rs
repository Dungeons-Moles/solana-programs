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

    /// Highest tier unlocked (0=free tier, 1=first paid tier, etc.)
    pub unlocked_tier: u8,

    /// Unix timestamp of profile creation
    pub created_at: i64,

    /// PDA bump seed
    pub bump: u8,
}

impl PlayerProfile {
    /// Seed prefix for PDA derivation
    pub const SEED_PREFIX: &'static [u8] = b"player";
}

/// Treasury account holding collected SOL from tier unlocks.
/// PDA Seeds: [b"treasury"]
#[account]
#[derive(InitSpace)]
pub struct Treasury {
    /// Authority that can withdraw funds
    pub admin: Pubkey,

    /// Total SOL collected (in lamports)
    pub total_collected: u64,

    /// PDA bump seed
    pub bump: u8,
}

impl Treasury {
    /// Seed prefix for PDA derivation
    pub const SEED_PREFIX: &'static [u8] = b"treasury";
}
