use anchor_lang::prelude::*;

/// Marketplace configuration (singleton)
/// PDA: [b"marketplace_config"]
#[account]
pub struct MarketplaceConfig {
    /// Admin authority
    pub authority: Pubkey,
    /// Skins collection address (Metaplex Core)
    pub skins_collection: Pubkey,
    /// Items collection address (Metaplex Core)
    pub items_collection: Pubkey,
    /// Company treasury for fee collection
    pub company_treasury: Pubkey,
    /// Gauntlet pool vault for fee collection
    pub gauntlet_pool: Pubkey,
    /// Company fee in basis points (e.g., 300 = 3%)
    pub company_fee_bps: u16,
    /// Gauntlet pool fee in basis points (e.g., 200 = 2%)
    pub gauntlet_fee_bps: u16,
    /// PDA bump
    pub bump: u8,
}

impl MarketplaceConfig {
    pub const SEED_PREFIX: &'static [u8] = b"marketplace_config";
    /// 32*5 + 2*2 + 1 = 165
    pub const INIT_SPACE: usize = 32 + 32 + 32 + 32 + 32 + 2 + 2 + 1;
}

/// NFT listing for sale
/// PDA: [b"listing", asset.key()]
#[account]
pub struct Listing {
    /// Seller wallet
    pub seller: Pubkey,
    /// Metaplex Core asset address
    pub asset: Pubkey,
    /// Collection the asset belongs to
    pub collection: Pubkey,
    /// Sale price in lamports
    pub price_lamports: u64,
    /// When the listing was created
    pub created_at: i64,
    /// PDA bump
    pub bump: u8,
}

impl Listing {
    pub const SEED_PREFIX: &'static [u8] = b"listing";
    /// 32*3 + 8 + 8 + 1 = 113
    pub const INIT_SPACE: usize = 32 + 32 + 32 + 8 + 8 + 1;
}

// ============================================================================
// Quest System State
// ============================================================================

/// Quest type (daily, weekly, seasonal)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum QuestType {
    Daily,
    Weekly,
    Seasonal,
}

/// Objective type for quests
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ObjectiveType {
    WinBattles,
    CompleteLevels,
    PlayPvpMatches,
    DefeatBosses,
    CollectGold,
}

/// Reward type for quest completion
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum RewardType {
    GauntletBooster,
    Skin,
    NftItem,
}

/// Quest template
/// PDA: [b"quest_def", &quest_id.to_le_bytes()]
#[account]
pub struct QuestDefinition {
    /// Unique quest identifier
    pub quest_id: u16,
    /// Daily, Weekly, or Seasonal
    pub quest_type: QuestType,
    /// What the player must do
    pub objective_type: ObjectiveType,
    /// How many times the objective must be completed
    pub objective_count: u16,
    /// What reward is given
    pub reward_type: RewardType,
    /// Encoded reward parameters (skin_id, item_id, booster count, etc.)
    pub reward_data: [u8; 32],
    /// Season number (0 = permanent, 1+ = seasonal)
    pub season: u8,
    /// Whether quest is currently active
    pub active: bool,
    /// PDA bump
    pub bump: u8,
}

impl QuestDefinition {
    pub const SEED_PREFIX: &'static [u8] = b"quest_def";
    /// 2 + 1 + 1 + 2 + 1 + 32 + 1 + 1 + 1 = 42
    pub const INIT_SPACE: usize = 2 + 1 + 1 + 2 + 1 + 32 + 1 + 1 + 1;
}

/// Player quest progress
/// PDA: [b"quest_progress", player.key(), &quest_id.to_le_bytes()]
#[account]
pub struct QuestProgress {
    /// Player wallet
    pub player: Pubkey,
    /// Quest ID reference
    pub quest_id: u16,
    /// Current progress towards objective
    pub progress: u16,
    /// Whether objective is completed
    pub completed: bool,
    /// Whether reward has been claimed
    pub claimed: bool,
    /// Last reset timestamp (for daily/weekly quests)
    pub last_reset: i64,
    /// PDA bump
    pub bump: u8,
}

impl QuestProgress {
    pub const SEED_PREFIX: &'static [u8] = b"quest_progress";
    /// 32 + 2 + 2 + 1 + 1 + 8 + 1 = 47
    pub const INIT_SPACE: usize = 32 + 2 + 2 + 1 + 1 + 8 + 1;
}
