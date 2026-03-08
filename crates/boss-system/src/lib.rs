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
pub use combat_system::state::CombatantInput;
pub use combat_system::{EffectType, ItemEffect, TriggerType};
use combat_system::state::{AnnotatedItemEffect, CombatSourceKind, CombatSourceRef};

/// Biome type for boss categorization
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Biome {
    A,
    B,
}

/// Week within a run (determines boss pool)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Week {
    One,
    Two,
    Three,
}

/// Boss stats structure
#[derive(Clone, Copy, Debug, Default)]
pub struct BossStats {
    pub hp: u16,
    pub atk: u8,
    pub arm: u8,
    pub spd: u8,
    pub dig: u8,
}

/// Item tags for weakness matching
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
#[derive(Clone, Debug)]
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
#[derive(Clone, Copy, Debug)]
pub struct ScaledBossStats {
    pub hp: u16,
    pub atk: u8,
    pub arm: u8,
    pub spd: u8,
    pub dig: u8,
    pub strikes: u8,
}

/// Returns the boss definition for a given stage and week.
pub fn get_boss_for_stage(stage: u8, week: Week) -> Result<BossInfo, BossSystemError> {
    if !(1..=40).contains(&stage) {
        return Err(BossSystemError::InvalidStage);
    }

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
pub fn get_scaled_boss_stats(stage: u8, week: Week) -> Result<ScaledBossStats, BossSystemError> {
    if !(1..=40).contains(&stage) {
        return Err(BossSystemError::InvalidStage);
    }

    let boss = select_boss(stage, week);
    let scaled = scale_boss(boss, stage, week);
    Ok(scaled)
}

/// Returns the two weakness tags for the boss at the given stage and week.
/// Used by item offer generation for tag weighting.
pub fn get_boss_weaknesses(stage: u8, week: Week) -> Result<[ItemTag; 2], BossSystemError> {
    if !(1..=40).contains(&stage) {
        return Err(BossSystemError::InvalidStage);
    }

    let boss = select_boss(stage, week);
    Ok(boss.weakness_tags)
}

/// Converts boss traits to ItemEffect array for combat system integration.
/// Returns all effects that should be active for this boss.
pub fn get_boss_effects(stage: u8, week: Week) -> Result<Vec<ItemEffect>, BossSystemError> {
    if !(1..=40).contains(&stage) {
        return Err(BossSystemError::InvalidStage);
    }

    let boss = select_boss(stage, week);
    Ok(get_boss_item_effects(boss))
}

pub fn get_boss_annotated_effects(
    stage: u8,
    week: Week,
) -> Result<Vec<AnnotatedItemEffect>, BossSystemError> {
    if !(1..=40).contains(&stage) {
        return Err(BossSystemError::InvalidStage);
    }

    let boss = select_boss(stage, week);
    Ok(get_boss_annotated_item_effects(boss))
}

/// Returns a complete CombatantInput struct for initializing boss combat.
/// Includes scaled stats ready for combat system.
pub fn get_boss_combat_input(stage: u8, week: Week) -> Result<CombatantInput, BossSystemError> {
    if !(1..=40).contains(&stage) {
        return Err(BossSystemError::InvalidStage);
    }

    let boss = select_boss(stage, week);
    let scaled = scale_boss(boss, stage, week);
    let mut combatant = to_combatant_input(&scaled);
    let mut source_id = [0u8; 16];
    source_id[..12].copy_from_slice(&boss.id);
    combatant.attack_source = Some(CombatSourceRef {
        kind: CombatSourceKind::Boss,
        id: source_id,
    });
    Ok(combatant)
}
