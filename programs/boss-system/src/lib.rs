use anchor_lang::prelude::*;

pub mod bosses;
pub mod errors;
pub mod scaling;
pub mod selection;
pub mod traits;

pub use bosses::*;
pub use errors::*;
pub use scaling::*;
pub use selection::*;
pub use traits::*;

// Re-export combat types from combat-system to avoid duplication
pub use combat_system::{EffectType, ItemEffect, TriggerType};

declare_id!("AjGZWfgKxaEx27YpnRxWqjbnANBDJSMHdJLWyrFWrvJY");

/// Biome type for boss categorization
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Biome {
    A,
    B,
}

/// Week within a run (determines boss pool)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Week {
    One,
    Two,
    Three,
}

/// Boss stats structure
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default)]
pub struct BossStats {
    pub hp: u16,
    pub atk: u8,
    pub arm: u8,
    pub spd: u8,
    pub dig: u8,
}

/// Item tags for weakness matching
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemTag {
    Stone,
    Scout,
    Greed,
    Blast,
    Frost,
    Rust,
    Blood,
    Tempo,
}

/// Summary information about a boss for external queries
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct BossInfo {
    pub id: [u8; 12],
    pub name: String,
    pub emoji: [u8; 4],
    pub biome: Biome,
    pub week: Week,
    pub boss_index: u8,
    pub weakness_tags: [ItemTag; 2],
    pub trait_count: u8,
}

/// Boss stats after applying stage/act scaling
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub struct ScaledBossStats {
    pub hp: u16,
    pub atk: u8,
    pub arm: u8,
    pub spd: u8,
    pub dig: u8,
    pub strikes: u8,
}

/// Input data for combat system
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub struct CombatantInput {
    pub hp: i16,
    pub max_hp: u16,
    pub atk: i16,
    pub arm: i16,
    pub spd: i16,
    pub dig: i16,
    pub strikes: u8,
}

#[program]
pub mod boss_system {
    use super::*;

    /// Returns the boss definition for a given stage and week.
    /// This is a view function that returns data without modifying state.
    pub fn get_boss_for_stage(_ctx: Context<Empty>, stage: u8, week: Week) -> Result<BossInfo> {
        require!((1..=80).contains(&stage), BossSystemError::InvalidStage);

        let boss = select_boss(stage, week);
        Ok(BossInfo {
            id: boss.id,
            name: boss.name.to_string(),
            emoji: boss.emoji,
            biome: boss.biome,
            week: boss.week,
            boss_index: boss.boss_index,
            weakness_tags: boss.weakness_tags,
            trait_count: boss.traits.len() as u8,
        })
    }

    /// Returns scaled boss stats for a specific stage and week.
    /// Applies tier scaling and act baseline bonuses.
    pub fn get_scaled_boss_stats(
        _ctx: Context<Empty>,
        stage: u8,
        week: Week,
    ) -> Result<ScaledBossStats> {
        require!((1..=80).contains(&stage), BossSystemError::InvalidStage);

        let boss = select_boss(stage, week);
        let scaled = scale_boss(boss, stage, week);
        Ok(scaled)
    }

    /// Returns the two weakness tags for the boss at the given stage and week.
    /// Used by item offer generation for tag weighting.
    pub fn get_boss_weaknesses(
        _ctx: Context<Empty>,
        stage: u8,
        week: Week,
    ) -> Result<[ItemTag; 2]> {
        require!((1..=80).contains(&stage), BossSystemError::InvalidStage);

        let boss = select_boss(stage, week);
        Ok(boss.weakness_tags)
    }

    /// Converts boss traits to ItemEffect array for combat system integration.
    /// Returns all effects that should be active for this boss.
    pub fn get_boss_effects(
        _ctx: Context<Empty>,
        stage: u8,
        week: Week,
    ) -> Result<Vec<ItemEffect>> {
        require!((1..=80).contains(&stage), BossSystemError::InvalidStage);

        let boss = select_boss(stage, week);
        Ok(get_boss_item_effects(boss))
    }

    /// Returns a complete CombatantInput struct for initializing boss combat.
    /// Includes scaled stats ready for combat system.
    pub fn get_boss_combat_input(
        _ctx: Context<Empty>,
        stage: u8,
        week: Week,
    ) -> Result<CombatantInput> {
        require!((1..=80).contains(&stage), BossSystemError::InvalidStage);

        let boss = select_boss(stage, week);
        let scaled = scale_boss(boss, stage, week);
        Ok(to_combatant_input(&scaled))
    }
}

#[derive(Accounts)]
pub struct Empty<'info> {
    /// CHECK: Empty accounts struct for view functions that don't need accounts
    #[account()]
    pub _placeholder: Option<AccountInfo<'info>>,
}
