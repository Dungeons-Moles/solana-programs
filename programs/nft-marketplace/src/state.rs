use quasar_lang::prelude::*;

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, QuasarSerialize)]
pub enum QuestType {
    Daily = 0,
    Weekly = 1,
    Seasonal = 2,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, QuasarSerialize)]
pub enum ObjectiveType {
    WinBattles = 0,
    CompleteLevels = 1,
    PlayPvpMatches = 2,
    DefeatBosses = 3,
    CollectGold = 4,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, QuasarSerialize)]
pub enum RewardType {
    GauntletBooster = 0,
    Skin = 1,
    NftItem = 2,
}

#[account(
    discriminator = [169, 22, 247, 131, 182, 200, 81, 124],
    fixed_capacity,
    set_inner
)]
#[seeds(b"marketplace_config")]
pub struct MarketplaceConfig {
    pub authority: Address,
    pub skins_collection: Address,
    pub items_collection: Address,
    pub company_treasury: Address,
    pub gauntlet_pool: Address,
    pub company_fee_bps: u16,
    pub gauntlet_fee_bps: u16,
    pub bump: u8,
}

impl MarketplaceConfig {
    pub const SEED_PREFIX: &'static [u8] = b"marketplace_config";
}

#[account(
    discriminator = [218, 32, 50, 73, 43, 134, 26, 58],
    fixed_capacity,
    set_inner
)]
#[seeds(b"listing", asset: Address)]
pub struct Listing {
    pub seller: Address,
    pub asset: Address,
    pub collection: Address,
    pub price_lamports: u64,
    pub created_at: i64,
    pub bump: u8,
}

impl Listing {
    pub const SEED_PREFIX: &'static [u8] = b"listing";
}

#[account(
    discriminator = [222, 160, 151, 133, 226, 88, 154, 91],
    fixed_capacity,
    set_inner
)]
#[seeds(b"relic_asset", asset: Address)]
pub struct RelicAsset {
    pub asset: Address,
    pub item_id: [u8; 8],
    pub bump: u8,
}

impl RelicAsset {
    pub const SEED_PREFIX: &'static [u8] = b"relic_asset";
}

#[account(
    discriminator = [106, 90, 250, 119, 170, 124, 111, 19],
    fixed_capacity,
    set_inner
)]
#[seeds(b"quest_def", quest_id: u16)]
pub struct QuestDefinition {
    pub quest_id: u16,
    pub quest_type: QuestType,
    pub objective_type: ObjectiveType,
    pub objective_count: u16,
    pub reward_type: RewardType,
    pub reward_data: [u8; 32],
    pub season: u8,
    pub active: bool,
    pub bump: u8,
}

impl QuestDefinition {
    pub const SEED_PREFIX: &'static [u8] = b"quest_def";
}

#[account(
    discriminator = [77, 66, 99, 169, 234, 177, 58, 162],
    fixed_capacity,
    set_inner
)]
#[seeds(b"quest_progress", player: Address, quest_id: u16)]
pub struct QuestProgress {
    pub player: Address,
    pub quest_id: u16,
    pub progress: u16,
    pub completed: bool,
    pub claimed: bool,
    pub last_reset: i64,
    pub bump: u8,
}

impl QuestProgress {
    pub const SEED_PREFIX: &'static [u8] = b"quest_progress";
}
