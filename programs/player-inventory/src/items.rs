//! Item definitions for the player-inventory program.
//!
//! Contains the core 80 item definitions as compile-time constants.
//! 16 Tools + 64 Gear across 8 tags, plus the special Basic Pickaxe starter tool.

use crate::state::{
    Condition, EffectDefinition, EffectType, ItemTag, ItemType, Rarity, StatusType, TriggerType,
};

/// Static item definition (compile-time constant)
#[derive(Clone, Copy, Debug)]
pub struct ItemDefinition {
    /// Unique ID (e.g., "T-ST-01")
    pub id: &'static [u8; 8],
    /// Display name (e.g., "Bulwark Shovel")
    pub name: &'static str,
    /// Tool or Gear
    pub item_type: ItemType,
    /// One of 8 tags (or None for special items)
    pub tag: ItemTag,
    /// Common, Rare, Heroic, or Mythic
    pub rarity: Rarity,
    /// Combat effects with tier-scaled values
    pub effects: &'static [EffectDefinition],
}

// =============================================================================
// BASIC PICKAXE - Starter Tool (T-XX-00)
// Special item that cannot be found in-game, only given at session start
// =============================================================================

/// The Basic Pickaxe is the starter tool given to all players.
/// It provides +1 ATK at battle start and cannot be found on the map.
/// ID format: T-XX-00 where XX indicates it's a special/starter item.
pub const BASIC_PICKAXE: ItemDefinition = ItemDefinition {
    id: b"T-XX-00\0",
    name: "Basic Pickaxe",
    item_type: ItemType::Tool,
    tag: ItemTag::None,
    rarity: Rarity::Common,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::GainAtk,
        false,
        [1, 1, 1], // Always +1 ATK regardless of tier (unfusable)
    )],
};

// =============================================================================
// STONE Tag Items (T-ST-01 through T-ST-02, G-ST-01 through G-ST-08)
// Theme: Armor, Shrapnel, durability
// =============================================================================

pub const T_ST_01: ItemDefinition = ItemDefinition {
    id: b"T-ST-01\0",
    name: "Bulwark Shovel",
    item_type: ItemType::Tool,
    tag: ItemTag::Stone,
    rarity: Rarity::Common,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [1, 2, 3],
        ),
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainArmor,
            false,
            [4, 6, 8],
        ),
    ],
};

pub const T_ST_02: ItemDefinition = ItemDefinition {
    id: b"T-ST-02\0",
    name: "Cragbreaker Hammer",
    item_type: ItemType::Tool,
    tag: ItemTag::Stone,
    rarity: Rarity::Rare,
    effects: &[
        // +2/3/4 ATK
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [2, 3, 4],
        ),
        // +3/5/7 ARM
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainArmor,
            false,
            [3, 5, 7],
        ),
        // First strike each turn removes 1/2/3 enemy Armor before damage
        EffectDefinition::new(TriggerType::OnHit, EffectType::RemoveArmor, true, [1, 2, 3]),
    ],
};

pub const G_ST_01: ItemDefinition = ItemDefinition {
    id: b"G-ST-01\0",
    name: "Miner Helmet",
    item_type: ItemType::Gear,
    tag: ItemTag::Stone,
    rarity: Rarity::Common,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::GainArmor,
        false,
        [3, 6, 9],
    )],
};

pub const G_ST_02: ItemDefinition = ItemDefinition {
    id: b"G-ST-02\0",
    name: "Work Vest",
    item_type: ItemType::Gear,
    tag: ItemTag::Stone,
    rarity: Rarity::Common,
    effects: &[
        // +4/8/12 HP (permanent max HP bonus)
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::MaxHp,
            false,
            [4, 8, 12],
        ),
        // +1 ARM (flat across all tiers per GDD)
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainArmor,
            false,
            [1, 1, 1],
        ),
    ],
};

pub const G_ST_03: ItemDefinition = ItemDefinition {
    id: b"G-ST-03\0",
    name: "Spiked Bracers",
    item_type: ItemType::Gear,
    tag: ItemTag::Stone,
    rarity: Rarity::Common, // GDD says Common, not Rare
    effects: &[
        // Battle Start: gain 2/4/6 Shrapnel
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::ApplyShrapnel,
            false,
            [2, 4, 6],
        ),
    ],
};

pub const G_ST_04: ItemDefinition = ItemDefinition {
    id: b"G-ST-04\0",
    name: "Reinforcement Plate",
    item_type: ItemType::Gear,
    tag: ItemTag::Stone,
    rarity: Rarity::Rare,
    effects: &[
        // Every other turn: gain 1/2/3 Armor
        EffectDefinition::new(
            TriggerType::EveryOtherTurn,
            EffectType::GainArmor,
            false,
            [1, 2, 3],
        ),
    ],
};

pub const G_ST_05: ItemDefinition = ItemDefinition {
    id: b"G-ST-05\0",
    name: "Rebar Carapace",
    item_type: ItemType::Gear,
    tag: ItemTag::Stone,
    rarity: Rarity::Rare,
    effects: &[
        // Exposed (once per battle): +4/6/8 ARM (stronger safety net)
        EffectDefinition::new(
            TriggerType::FirstTimeExposed,
            EffectType::GainArmor,
            false,
            [4, 6, 8],
        ),
    ],
};

pub const G_ST_06: ItemDefinition = ItemDefinition {
    id: b"G-ST-06\0",
    name: "Shrapnel Talisman",
    item_type: ItemType::Gear,
    tag: ItemTag::Stone,
    rarity: Rarity::Rare, // GDD says Rare
    effects: &[
        // First time you gain Shrapnel this battle: gain 2/3/4 Armor
        EffectDefinition::new(
            TriggerType::FirstTimeGainShrapnel,
            EffectType::GainArmor,
            false,
            [2, 3, 4],
        ),
    ],
};

pub const G_ST_07: ItemDefinition = ItemDefinition {
    id: b"G-ST-07\0",
    name: "Crystal Crown",
    item_type: ItemType::Gear,
    tag: ItemTag::Stone,
    rarity: Rarity::Mythic,
    effects: &[
        // Battle Start: gain Max HP equal to your starting Armor (cap 10/15/20)
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::ArmorToMaxHp,
            false,
            [10, 15, 20], // Cap values per tier
        ),
    ],
};

pub const G_ST_08: ItemDefinition = ItemDefinition {
    id: b"G-ST-08\0",
    name: "Stone Sigil",
    item_type: ItemType::Gear,
    tag: ItemTag::Stone,
    rarity: Rarity::Heroic, // GDD says Heroic
    effects: &[
        // End of turn: if you have >=3 Armor, gain +1/2/3 Armor
        EffectDefinition::with_condition(
            TriggerType::TurnEnd,
            EffectType::GainArmor,
            false,
            [1, 2, 3],
            Condition::OwnerArmorAtLeast(3),
        ),
    ],
};

// =============================================================================
// SCOUT Tag Items (T-SC-01, T-SC-02, G-SC-01 through G-SC-08)
// Theme: DIG, multi-strike, mobility
// =============================================================================

pub const T_SC_01: ItemDefinition = ItemDefinition {
    id: b"T-SC-01\0",
    name: "Twin Picks",
    item_type: ItemType::Tool,
    tag: ItemTag::Scout,
    rarity: Rarity::Common,
    effects: &[
        // +1/2/3 ATK
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [1, 2, 3],
        ),
        // Strike 2 times per turn (base 1 + 1 = 2)
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainStrikes,
            false,
            [1, 1, 1], // Flat +1 strike at all tiers
        ),
    ],
};

pub const T_SC_02: ItemDefinition = ItemDefinition {
    id: b"T-SC-02\0",
    name: "Pneumatic Drill",
    item_type: ItemType::Tool,
    tag: ItemTag::Scout,
    rarity: Rarity::Rare,
    effects: &[
        // +1/2/3 ATK
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [1, 2, 3],
        ),
        // Strike 3 times per turn (base 1 + 2 = 3)
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainStrikes,
            false,
            [2, 2, 2], // Flat +2 strikes at all tiers
        ),
        // Strikes beyond the 2nd use only 50% of gear-derived ATK bonus.
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::HalfGearAtkAfterSecondStrike,
            false,
            [1, 1, 1],
        ),
    ],
};

pub const G_SC_01: ItemDefinition = ItemDefinition {
    id: b"G-SC-01\0",
    name: "Miner Boots",
    item_type: ItemType::Gear,
    tag: ItemTag::Scout,
    rarity: Rarity::Common,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::GainDig,
        false,
        [2, 3, 4],
    )],
};

pub const G_SC_02: ItemDefinition = ItemDefinition {
    id: b"G-SC-02\0",
    name: "Leather Gloves",
    item_type: ItemType::Gear,
    tag: ItemTag::Scout,
    rarity: Rarity::Common,
    effects: &[
        // +1/2/3 ATK per GDD
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [1, 2, 3],
        ),
        // +1 DIG (flat) per GDD
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainDig,
            false,
            [1, 1, 1],
        ),
    ],
};

pub const G_SC_03: ItemDefinition = ItemDefinition {
    id: b"G-SC-03\0",
    name: "Tunnel Instinct",
    item_type: ItemType::Gear,
    tag: ItemTag::Scout,
    rarity: Rarity::Rare,
    effects: &[
        // Battle Start: if DIG > enemy DIG, gain +1/2/3 SPD (this battle)
        EffectDefinition::with_condition(
            TriggerType::BattleStart,
            EffectType::GainSpd,
            false,
            [1, 2, 3],
            Condition::DigGreaterThanEnemyDig,
        ),
    ],
};

pub const G_SC_04: ItemDefinition = ItemDefinition {
    id: b"G-SC-04\0",
    name: "Tunneler Spurs",
    item_type: ItemType::Gear,
    tag: ItemTag::Scout,
    rarity: Rarity::Rare,
    effects: &[
        // +1/2/3 SPD
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainSpd,
            false,
            [1, 2, 3],
        ),
        // If you act first on Turn 1, gain +1/2/3 DIG (this battle)
        EffectDefinition::new(
            TriggerType::FirstTurnIfFaster,
            EffectType::GainDig,
            false,
            [1, 2, 3],
        ),
    ],
};

pub const G_SC_05: ItemDefinition = ItemDefinition {
    id: b"G-SC-05\0",
    name: "Wall-Sense Visor",
    item_type: ItemType::Gear,
    tag: ItemTag::Scout,
    rarity: Rarity::Rare,
    effects: &[
        // +1/2/3 DIG
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainDig,
            false,
            [1, 2, 3],
        ),
        // Battle Start: if DIG > enemy DIG, gain +2/3/4 Armor
        EffectDefinition::with_condition(
            TriggerType::BattleStart,
            EffectType::GainArmor,
            false,
            [2, 3, 4],
            Condition::DigGreaterThanEnemyDig,
        ),
    ],
};

pub const G_SC_06: ItemDefinition = ItemDefinition {
    id: b"G-SC-06\0",
    name: "Drill Servo",
    item_type: ItemType::Gear,
    tag: ItemTag::Scout,
    rarity: Rarity::Heroic,
    effects: &[
        // Wounded: gain +1/2/3 additional strikes (this battle)
        EffectDefinition::new(
            TriggerType::Wounded,
            EffectType::GainStrikes,
            true, // once per turn to prevent stacking every check
            [1, 1, 2],
        ),
    ],
};

pub const G_SC_07: ItemDefinition = ItemDefinition {
    id: b"G-SC-07\0",
    name: "Weak-Point Manual",
    item_type: ItemType::Gear,
    tag: ItemTag::Scout,
    rarity: Rarity::Heroic,
    effects: &[
        // If DIG > enemy Armor: your strikes ignore 1/2/3 Armor (this battle)
        // Note: The condition checks DIG > enemy ARM at battle start
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::SetArmorPiercing,
            false,
            [1, 2, 3],
        ),
    ],
};

pub const G_SC_08: ItemDefinition = ItemDefinition {
    id: b"G-SC-08\0",
    name: "Gear-Link Medallion",
    item_type: ItemType::Gear,
    tag: ItemTag::Scout,
    rarity: Rarity::Mythic,
    effects: &[
        // On Hit effects trigger twice (once/turn).
        // Source scoping is enforced by higher-level combat resolution.
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::DoubleOnHitEffects,
            false,
            [1, 1, 1], // Toggle flag, stays flat
        ),
        // Tier bonus: better medallion = faster reaction
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainSpd,
            false,
            [0, 1, 2],
        ),
    ],
};

// =============================================================================
// GREED Tag Items (T-GR-01, T-GR-02, G-GR-01 through G-GR-08)
// Theme: Gold generation, shard effects
// =============================================================================

pub const T_GR_01: ItemDefinition = ItemDefinition {
    id: b"T-GR-01\0",
    name: "Glittering Pick",
    item_type: ItemType::Tool,
    tag: ItemTag::Greed,
    rarity: Rarity::Common,
    effects: &[
        // +1/2/3 ATK
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [1, 2, 3],
        ),
        // On Hit (once/turn): gain 1 Gold (flat, per GDD)
        EffectDefinition::new(TriggerType::OnHit, EffectType::GainGold, true, [1, 1, 1]),
        // Victory: gain +2 Gold
        EffectDefinition::new(TriggerType::Victory, EffectType::GainGold, false, [2, 2, 2]),
    ],
};

pub const T_GR_02: ItemDefinition = ItemDefinition {
    id: b"T-GR-02\0",
    name: "Gemfinder Staff",
    item_type: ItemType::Tool,
    tag: ItemTag::Greed,
    rarity: Rarity::Heroic,
    effects: &[
        // +1/1/2 ATK
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [1, 1, 2],
        ),
        // +1/2/2 ARM
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainArmor,
            false,
            [1, 2, 2],
        ),
        // +1/1/2 DIG
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainDig,
            false,
            [1, 1, 2],
        ),
        // First hit each turn triggers all your Shard effects
        EffectDefinition::new(
            TriggerType::OnHit,
            EffectType::TriggerAllShards,
            true, // once per turn (first hit only)
            [1, 1, 1], // Toggle flag, stays flat
        ),
    ],
};

pub const G_GR_01: ItemDefinition = ItemDefinition {
    id: b"G-GR-01\0",
    name: "Loose Nuggets",
    item_type: ItemType::Gear,
    tag: ItemTag::Greed,
    rarity: Rarity::Common,
    effects: &[
        // Start of each Day: gain 3/6/9 Gold
        EffectDefinition::new(
            TriggerType::DayStart,
            EffectType::GainGold,
            false,
            [3, 6, 9],
        ),
    ],
};

pub const G_GR_02: ItemDefinition = ItemDefinition {
    id: b"G-GR-02\0",
    name: "Lucky Coin",
    item_type: ItemType::Gear,
    tag: ItemTag::Greed,
    rarity: Rarity::Common,
    effects: &[
        // Victory: gain 2/4/6 Gold
        EffectDefinition::new(TriggerType::Victory, EffectType::GainGold, false, [2, 4, 6]),
        // Victory: heal 2/3/4 HP
        EffectDefinition::new(TriggerType::Victory, EffectType::Heal, false, [2, 3, 4]),
    ],
};

pub const G_GR_03: ItemDefinition = ItemDefinition {
    id: b"G-GR-03\0",
    name: "Gilded Band",
    item_type: ItemType::Gear,
    tag: ItemTag::Greed,
    rarity: Rarity::Heroic,
    effects: &[
        // Battle Start: gain Armor equal to floor(Gold/8) (cap 4/5/6)
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GoldToArmorScaled,
            false,
            [4, 5, 6], // Cap values per tier
        ),
    ],
};

pub const G_GR_04: ItemDefinition = ItemDefinition {
    id: b"G-GR-04\0",
    name: "Royal Bracer",
    item_type: ItemType::Gear,
    tag: ItemTag::Greed,
    rarity: Rarity::Mythic,
    effects: &[
        // Turn Start: convert 1 Gold → 3/4/5 Armor
        EffectDefinition::new(
            TriggerType::TurnStart,
            EffectType::ConsumeGoldForArmor,
            false,
            [3, 4, 5], // Armor gained per 1 gold consumed
        ),
    ],
};

pub const G_GR_05: ItemDefinition = ItemDefinition {
    id: b"G-GR-05\0",
    name: "Emerald Shard",
    item_type: ItemType::Gear,
    tag: ItemTag::Greed,
    rarity: Rarity::Common, // GDD says Common
    effects: &[
        // Every other turn (on first hit): heal 1/2/3 HP
        EffectDefinition::new(
            TriggerType::EveryOtherTurnFirstHit,
            EffectType::Heal,
            true, // once per turn (first hit only)
            [1, 2, 3],
        ),
    ],
};

pub const G_GR_06: ItemDefinition = ItemDefinition {
    id: b"G-GR-06\0",
    name: "Ruby Shard",
    item_type: ItemType::Gear,
    tag: ItemTag::Greed,
    rarity: Rarity::Common, // GDD says Common
    effects: &[
        // Every other turn (on first hit): deal 1/2/3 non-weapon damage
        EffectDefinition::new(
            TriggerType::EveryOtherTurnFirstHit,
            EffectType::DealNonWeaponDamage,
            true, // once per turn (first hit only)
            [1, 2, 3],
        ),
    ],
};

pub const G_GR_07: ItemDefinition = ItemDefinition {
    id: b"G-GR-07\0",
    name: "Sapphire Shard",
    item_type: ItemType::Gear,
    tag: ItemTag::Greed,
    rarity: Rarity::Common, // GDD says Common
    effects: &[
        // Every other turn (on first hit): gain +1/2/3 ARM
        EffectDefinition::new(
            TriggerType::EveryOtherTurnFirstHit,
            EffectType::GainArmor,
            true, // once per turn (first hit only)
            [1, 2, 3],
        ),
    ],
};

pub const G_GR_08: ItemDefinition = ItemDefinition {
    id: b"G-GR-08\0",
    name: "Citrine Shard",
    item_type: ItemType::Gear,
    tag: ItemTag::Greed,
    rarity: Rarity::Common, // GDD says Common
    effects: &[
        // Every other turn (on first hit): gain +1/2/3 Gold
        EffectDefinition::new(
            TriggerType::EveryOtherTurnFirstHit,
            EffectType::GainGold,
            true, // once per turn (first hit only)
            [1, 2, 3],
        ),
    ],
};

// =============================================================================
// BLAST Tag Items (T-BL-01, T-BL-02, G-BL-01 through G-BL-08)
// Theme: Countdown bombs, non-weapon damage
// =============================================================================

pub const T_BL_01: ItemDefinition = ItemDefinition {
    id: b"T-BL-01\0",
    name: "Fuse Pick",
    item_type: ItemType::Tool,
    tag: ItemTag::Blast,
    rarity: Rarity::Common,
    effects: &[
        // +1/2/3 ATK per GDD
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [1, 2, 3],
        ),
        // First hit: 1/2/2 non-weapon damage (once per turn)
        EffectDefinition::new(
            TriggerType::OnHit,
            EffectType::DealNonWeaponDamage,
            true,
            [1, 2, 2],
        ),
    ],
};

pub const T_BL_02: ItemDefinition = ItemDefinition {
    id: b"T-BL-02\0",
    name: "Spark Pick",
    item_type: ItemType::Tool,
    tag: ItemTag::Blast,
    rarity: Rarity::Rare,
    effects: &[
        // +1/2/3 ATK per GDD
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [1, 2, 3],
        ),
        // On Hit (once/turn): reduce your highest Countdown by 1
        EffectDefinition::new(
            TriggerType::OnHit,
            EffectType::ReduceAllCountdowns,
            true,      // once per turn
            [1, 1, 1], // Reduce by 1
        ),
    ],
};

pub const G_BL_01: ItemDefinition = ItemDefinition {
    id: b"G-BL-01\0",
    name: "Small Charge",
    item_type: ItemType::Gear,
    tag: ItemTag::Blast,
    rarity: Rarity::Common,
    effects: &[
        // Countdown(2): deal 10/12/14 to enemy (non-weapon)
        EffectDefinition::new(
            TriggerType::Countdown { turns: 2 },
            EffectType::DealNonWeaponDamage,
            false,
            [10, 12, 14],
        ),
        // Also deal 4/5/6 to self (non-weapon)
        EffectDefinition::new(
            TriggerType::Countdown { turns: 2 },
            EffectType::DealSelfNonWeaponDamage,
            false,
            [4, 5, 6],
        ),
    ],
};

pub const G_BL_02: ItemDefinition = ItemDefinition {
    id: b"G-BL-02\0",
    name: "Blast Suit",
    item_type: ItemType::Gear,
    tag: ItemTag::Blast,
    rarity: Rarity::Rare,
    effects: &[
        // You ignore damage from your own BLAST items
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::BlastImmunity,
            false,
            [1, 1, 1], // Value doesn't matter, just enables immunity
        ),
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainArmor,
            false,
            [2, 3, 4],
        ),
    ],
};

pub const G_BL_03: ItemDefinition = ItemDefinition {
    id: b"G-BL-03\0",
    name: "Explosive Powder",
    item_type: ItemType::Gear,
    tag: ItemTag::Blast,
    rarity: Rarity::Rare,
    effects: &[
        // Your non-weapon damage deals +1/2/3
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::AmplifyNonWeaponDamage,
            false,
            [1, 2, 3],
        ),
    ],
};

pub const G_BL_04: ItemDefinition = ItemDefinition {
    id: b"G-BL-04\0",
    name: "Double Detonation",
    item_type: ItemType::Gear,
    tag: ItemTag::Blast,
    rarity: Rarity::Rare,
    effects: &[
        // TODO: GDD says "Second time you deal non-weapon damage each turn: deal +2/3/4 more"
        // This needs tracking of non-weapon damage count per turn
        // Placeholder: deal extra non-weapon damage on hit
        EffectDefinition::new(
            TriggerType::OnHit,
            EffectType::DealNonWeaponDamage,
            true,
            [2, 3, 4],
        ),
    ],
};

pub const G_BL_05: ItemDefinition = ItemDefinition {
    id: b"G-BL-05\0",
    name: "Bomb Satchel",
    item_type: ItemType::Gear,
    tag: ItemTag::Blast,
    rarity: Rarity::Heroic,
    effects: &[
        // Battle Start: reduce Countdown of all your bomb items by 1 (min 0)
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::ReduceAllCountdowns,
            false,
            [1, 1, 1], // Stays flat (scaling to 2 would make countdown-2 bombs instant)
        ),
        // Tier bonus: sturdier satchel gives blast protection
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainArmor,
            false,
            [0, 1, 2],
        ),
    ],
};

pub const G_BL_06: ItemDefinition = ItemDefinition {
    id: b"G-BL-06\0",
    name: "Kindling Charge",
    item_type: ItemType::Gear,
    tag: ItemTag::Blast,
    rarity: Rarity::Rare,
    effects: &[
        // Battle Start: deal 2/3/4 non-weapon damage
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::DealNonWeaponDamage,
            false,
            [2, 3, 4],
        ),
        // Your next bomb this battle deals +3/5/7.
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::EmpowerNextBombDamage,
            false,
            [3, 5, 7],
        ),
        // Your next bomb self-damage is reduced by 2/3/4.
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::ReduceNextBombSelfDamage,
            false,
            [2, 3, 4],
        ),
    ],
};

pub const G_BL_07: ItemDefinition = ItemDefinition {
    id: b"G-BL-07\0",
    name: "Time Charge",
    item_type: ItemType::Gear,
    tag: ItemTag::Blast,
    rarity: Rarity::Heroic,
    effects: &[
        // Turn Start: gain +1/2/3 stored damage (this battle)
        EffectDefinition::new(
            TriggerType::TurnStart,
            EffectType::StoreDamage,
            false,
            [1, 2, 3],
        ),
        // StoreDamage is released automatically when first exposed by combat-system logic.
    ],
};

pub const G_BL_08: ItemDefinition = ItemDefinition {
    id: b"G-BL-08\0",
    name: "Twin-Fuse Knot",
    item_type: ItemType::Gear,
    tag: ItemTag::Blast,
    rarity: Rarity::Mythic,
    effects: &[
        // Your bomb triggers happen twice
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::DoubleBombTrigger,
            false,
            [1, 1, 1], // Toggle flag, stays flat
        ),
        // Tier bonus: refined fuse engineering = less backlash from double detonation
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::ReduceNextBombSelfDamage,
            false,
            [0, 1, 2],
        ),
    ],
};

// =============================================================================
// FROST Tag Items (T-FR-01 through T-FR-02, G-FR-01 through G-FR-08)
// Theme: Chill, SPD manipulation
// =============================================================================

pub const T_FR_01: ItemDefinition = ItemDefinition {
    id: b"T-FR-01\0",
    name: "Rime Pike",
    item_type: ItemType::Tool,
    tag: ItemTag::Frost,
    rarity: Rarity::Common,
    effects: &[
        // +1/2/3 ATK
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [1, 2, 3],
        ),
        // On Hit (once/turn): apply 1 Chill (flat, not scaling)
        EffectDefinition::new(TriggerType::OnHit, EffectType::ApplyChill, true, [1, 1, 1]),
        // On Hit (once/turn): if enemy has Chill, deal +1 bonus damage
        EffectDefinition::with_condition(
            TriggerType::OnHit,
            EffectType::DealDamage,
            true,
            [1, 1, 1],
            Condition::EnemyHasStatus(StatusType::Chill),
        ),
    ],
};

pub const T_FR_02: ItemDefinition = ItemDefinition {
    id: b"T-FR-02\0",
    name: "Glacier Fang",
    item_type: ItemType::Tool,
    tag: ItemTag::Frost,
    rarity: Rarity::Rare,
    effects: &[
        // +2/3/4 ATK
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [2, 3, 4],
        ),
        // On Hit (once/turn): apply 1 Chill
        EffectDefinition::new(TriggerType::OnHit, EffectType::ApplyChill, true, [1, 1, 1]),
        // If enemy has Chill, gain +1 SPD this turn (checked via condition)
        EffectDefinition::with_condition(
            TriggerType::OnHit,
            EffectType::GainSpd,
            true,
            [1, 1, 1],
            Condition::EnemyHasStatus(StatusType::Chill),
        ),
        // If enemy has Chill, deal +1 bonus damage
        EffectDefinition::with_condition(
            TriggerType::OnHit,
            EffectType::DealDamage,
            true,
            [1, 1, 1],
            Condition::EnemyHasStatus(StatusType::Chill),
        ),
    ],
};

pub const G_FR_01: ItemDefinition = ItemDefinition {
    id: b"G-FR-01\0",
    name: "Frost Lantern",
    item_type: ItemType::Gear,
    tag: ItemTag::Frost,
    rarity: Rarity::Common,
    effects: &[
        // Battle Start: give enemy 1/2/3 Chill
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::ApplyChill,
            false,
            [1, 2, 3],
        ),
    ],
};

pub const G_FR_02: ItemDefinition = ItemDefinition {
    id: b"G-FR-02\0",
    name: "Frostguard Buckler",
    item_type: ItemType::Gear,
    tag: ItemTag::Frost,
    rarity: Rarity::Heroic,
    effects: &[
        // +8/10/12 ARM
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainArmor,
            false,
            [8, 10, 12],
        ),
        // Battle Start: if enemy has Chill, gain +3/4/5 Armor
        EffectDefinition::with_condition(
            TriggerType::BattleStart,
            EffectType::GainArmor,
            false,
            [3, 4, 5],
            Condition::EnemyHasStatus(StatusType::Chill),
        ),
        // Battle Start: if enemy has Chill, apply 1 Chill
        EffectDefinition::with_condition(
            TriggerType::BattleStart,
            EffectType::ApplyChill,
            false,
            [1, 1, 1],
            Condition::EnemyHasStatus(StatusType::Chill),
        ),
    ],
};

pub const G_FR_03: ItemDefinition = ItemDefinition {
    id: b"G-FR-03\0",
    name: "Cold Snap Charm",
    item_type: ItemType::Gear,
    tag: ItemTag::Frost,
    rarity: Rarity::Rare,
    effects: &[
        // If you act first on Turn 1: apply 2/3/4 Chill
        EffectDefinition::new(
            TriggerType::FirstTurnIfFaster,
            EffectType::ApplyChill,
            false,
            [2, 3, 4],
        ),
    ],
};

pub const G_FR_04: ItemDefinition = ItemDefinition {
    id: b"G-FR-04\0",
    name: "Ice Skates",
    item_type: ItemType::Gear,
    tag: ItemTag::Frost,
    rarity: Rarity::Rare,
    effects: &[
        // +1/2/3 SPD
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainSpd,
            false,
            [1, 2, 3],
        ),
        // Exploration utility approximation: +1/1/1 DIG in combat systems.
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainDig,
            false,
            [1, 1, 1],
        ),
    ],
};

pub const G_FR_05: ItemDefinition = ItemDefinition {
    id: b"G-FR-05\0",
    name: "Rime Cloak",
    item_type: ItemType::Gear,
    tag: ItemTag::Frost,
    rarity: Rarity::Rare,
    effects: &[
        // +3/5/7 ARM
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainArmor,
            false,
            [3, 5, 7],
        ),
        // When struck (once/turn): apply 1 Chill to attacker
        EffectDefinition::new(
            TriggerType::OnStruck,
            EffectType::ApplyChill,
            true,
            [1, 1, 1],
        ),
    ],
};

pub const G_FR_06: ItemDefinition = ItemDefinition {
    id: b"G-FR-06\0",
    name: "Permafrost Core",
    item_type: ItemType::Gear,
    tag: ItemTag::Frost,
    rarity: Rarity::Mythic,
    effects: &[
        // Turn Start: if enemy has Chill, gain 2/3/4 Armor
        EffectDefinition::with_condition(
            TriggerType::TurnStart,
            EffectType::GainArmor,
            false,
            [2, 3, 4],
            Condition::EnemyHasStatus(StatusType::Chill),
        ),
        // Turn Start: if enemy has Chill, deal 2 non-weapon damage
        EffectDefinition::with_condition(
            TriggerType::TurnStart,
            EffectType::DealNonWeaponDamage,
            false,
            [2, 2, 2],
            Condition::EnemyHasStatus(StatusType::Chill),
        ),
    ],
};

pub const G_FR_07: ItemDefinition = ItemDefinition {
    id: b"G-FR-07\0",
    name: "Cold Front Idol",
    item_type: ItemType::Gear,
    tag: ItemTag::Frost,
    rarity: Rarity::Heroic,
    effects: &[
        // Every other turn: apply 1 Chill (status application, stays flat)
        EffectDefinition::new(
            TriggerType::EveryOtherTurn,
            EffectType::ApplyChill,
            false,
            [1, 1, 1],
        ),
        // Every other turn: deal 1/2/3 non-weapon damage
        EffectDefinition::new(
            TriggerType::EveryOtherTurn,
            EffectType::DealNonWeaponDamage,
            false,
            [1, 2, 3],
        ),
        // If enemy already has Chill, gain +1/1/2 SPD this turn
        EffectDefinition::with_condition(
            TriggerType::EveryOtherTurn,
            EffectType::GainSpd,
            false,
            [1, 1, 2],
            Condition::EnemyHasStatus(StatusType::Chill),
        ),
    ],
};

pub const G_FR_08: ItemDefinition = ItemDefinition {
    id: b"G-FR-08\0",
    name: "Deep Freeze Charm",
    item_type: ItemType::Gear,
    tag: ItemTag::Frost,
    rarity: Rarity::Heroic, // GDD says Heroic, not Mythic
    effects: &[
        // Wounded: apply 2/3/4 Chill
        EffectDefinition::new(
            TriggerType::Wounded,
            EffectType::ApplyChill,
            true,
            [2, 3, 4],
        ),
        // Wounded: reduce enemy SPD by 1 (this battle)
        EffectDefinition::new(
            TriggerType::Wounded,
            EffectType::ReduceEnemySpd,
            true,
            [1, 1, 1],
        ),
        // Approximation of "enemy takes +1 while chilled": amplify non-weapon damage.
        EffectDefinition::new(
            TriggerType::Wounded,
            EffectType::AmplifyNonWeaponDamage,
            true,
            [1, 1, 1],
        ),
    ],
};

// =============================================================================
// RUST Tag Items (T-RU-01 through T-RU-02, G-RU-01 through G-RU-08)
// Theme: Armor destruction
// =============================================================================

pub const T_RU_01: ItemDefinition = ItemDefinition {
    id: b"T-RU-01\0",
    name: "Corrosive Pick",
    item_type: ItemType::Tool,
    tag: ItemTag::Rust,
    rarity: Rarity::Common,
    effects: &[
        // +1/2/3 ATK
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [1, 2, 3],
        ),
        // On Hit (once/turn): apply 1 Rust (flat, not scaling)
        EffectDefinition::new(TriggerType::OnHit, EffectType::ApplyRust, true, [1, 1, 1]),
    ],
};

pub const T_RU_02: ItemDefinition = ItemDefinition {
    id: b"T-RU-02\0",
    name: "Etched Burrowblade",
    item_type: ItemType::Tool,
    tag: ItemTag::Rust,
    rarity: Rarity::Heroic,
    effects: &[
        // +2/3/4 ATK
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [2, 3, 4],
        ),
        // +2/3/4 SPD
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainSpd,
            false,
            [2, 3, 4],
        ),
        // If enemy has Rust, your strikes ignore 2/3/4 Armor
        EffectDefinition::with_condition(
            TriggerType::BattleStart,
            EffectType::SetArmorPiercing,
            false,
            [2, 3, 4],
            Condition::EnemyHasStatus(StatusType::Rust),
        ),
        // If enemy has >=4 Rust, ignore all Armor.
        EffectDefinition::with_condition(
            TriggerType::OnHit,
            EffectType::SetArmorPiercing,
            true,
            [32767, 32767, 32767],
            Condition::EnemyHasStatusAtLeast(StatusType::Rust, 4),
        ),
    ],
};

pub const G_RU_01: ItemDefinition = ItemDefinition {
    id: b"G-RU-01\0",
    name: "Oxidizer Vial",
    item_type: ItemType::Gear,
    tag: ItemTag::Rust,
    rarity: Rarity::Common,
    effects: &[
        // Battle Start: apply 1/2/3 Rust
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::ApplyRust,
            false,
            [1, 2, 3],
        ),
        // If enemy has Armor, apply +1 more Rust
        EffectDefinition::with_condition(
            TriggerType::BattleStart,
            EffectType::ApplyRust,
            false,
            [1, 1, 1],
            Condition::EnemyHasArmor,
        ),
    ],
};

pub const G_RU_02: ItemDefinition = ItemDefinition {
    id: b"G-RU-02\0",
    name: "Rust Spike",
    item_type: ItemType::Gear,
    tag: ItemTag::Rust,
    rarity: Rarity::Rare, // GDD says Rare
    effects: &[
        // On Hit (once/turn): apply 1 Rust (flat, not scaling)
        EffectDefinition::new(TriggerType::OnHit, EffectType::ApplyRust, true, [1, 1, 1]),
        // If enemy has Rust >= 3, deal bonus non-weapon damage
        EffectDefinition::with_condition(
            TriggerType::OnHit,
            EffectType::DealNonWeaponDamage,
            true,
            [1, 2, 2],
            Condition::EnemyHasStatusAtLeast(StatusType::Rust, 3),
        ),
    ],
};

pub const G_RU_03: ItemDefinition = ItemDefinition {
    id: b"G-RU-03\0",
    name: "Corroded Greaves",
    item_type: ItemType::Gear,
    tag: ItemTag::Rust,
    rarity: Rarity::Rare,
    effects: &[
        // +1/2/3 SPD per GDD
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainSpd,
            false,
            [1, 2, 3],
        ),
        // Wounded: 2/3/4 Rust per GDD
        EffectDefinition::new(TriggerType::Wounded, EffectType::ApplyRust, true, [2, 3, 4]),
    ],
};

pub const G_RU_04: ItemDefinition = ItemDefinition {
    id: b"G-RU-04\0",
    name: "Acid Phial",
    item_type: ItemType::Gear,
    tag: ItemTag::Rust,
    rarity: Rarity::Rare,
    effects: &[
        // BattleStart: -2/3/4 enemy ARM per GDD
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::RemoveArmor,
            false,
            [2, 3, 4],
        ),
    ],
};

pub const G_RU_05: ItemDefinition = ItemDefinition {
    id: b"G-RU-05\0",
    name: "Flaking Plating",
    item_type: ItemType::Gear,
    tag: ItemTag::Rust,
    rarity: Rarity::Rare,
    effects: &[
        // +6/8/10 ARM per GDD
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainArmor,
            false,
            [6, 8, 10],
        ),
        // Exposed: 2/3/4 Rust per GDD
        EffectDefinition::new(TriggerType::Exposed, EffectType::ApplyRust, true, [2, 3, 4]),
    ],
};

pub const G_RU_06: ItemDefinition = ItemDefinition {
    id: b"G-RU-06\0",
    name: "Rust Engine",
    item_type: ItemType::Gear,
    tag: ItemTag::Rust,
    rarity: Rarity::Heroic,
    effects: &[
        // Turn Start: if enemy has Rust OR no Armor, deal 1/2/3 non-weapon damage
        EffectDefinition::with_condition(
            TriggerType::TurnStart,
            EffectType::DealNonWeaponDamage,
            false,
            [1, 2, 3],
            Condition::EnemyHasStatusOrNoArmor(StatusType::Rust),
        ),
    ],
};

pub const G_RU_07: ItemDefinition = ItemDefinition {
    id: b"G-RU-07\0",
    name: "Corrosion Loop",
    item_type: ItemType::Gear,
    tag: ItemTag::Rust,
    rarity: Rarity::Mythic,
    effects: &[
        // OnHit (1/turn): apply +2/3/4 additional Rust
        EffectDefinition::new(TriggerType::OnHit, EffectType::ApplyRust, true, [2, 3, 4]),
        // OnHit (1/turn): if enemy has 0 Armor, deal 2/3/4 non-weapon damage
        EffectDefinition::with_condition(
            TriggerType::OnHit,
            EffectType::DealNonWeaponDamage,
            true,
            [2, 3, 4],
            Condition::EnemyHasNoArmor,
        ),
    ],
};

pub const G_RU_08: ItemDefinition = ItemDefinition {
    id: b"G-RU-08\0",
    name: "Salvage Clamp",
    item_type: ItemType::Gear,
    tag: ItemTag::Rust,
    rarity: Rarity::Common,
    effects: &[
        // Whenever you apply Rust (once/turn): gain 1/2/3 Gold
        EffectDefinition::new(
            TriggerType::OnApplyRust,
            EffectType::GainGold,
            true,      // once per turn
            [1, 2, 3],
        ),
        // Battle Start: if enemy has no Armor, apply 1 Rust anyway.
        EffectDefinition::with_condition(
            TriggerType::BattleStart,
            EffectType::ApplyRust,
            false,
            [1, 1, 1],
            Condition::EnemyHasNoArmor,
        ),
    ],
};

// =============================================================================
// BLOOD Tag Items (T-BO-01, T-BO-02, G-BO-01 through G-BO-08)
// Theme: Bleed, sustain
// =============================================================================

pub const T_BO_01: ItemDefinition = ItemDefinition {
    id: b"T-BO-01\0",
    name: "Serrated Drill",
    item_type: ItemType::Tool,
    tag: ItemTag::Blood,
    rarity: Rarity::Common,
    effects: &[
        // +1/2/3 ATK
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [1, 2, 3],
        ),
        // On Hit (once/turn): apply 1 Bleed (flat, not scaling)
        EffectDefinition::new(TriggerType::OnHit, EffectType::ApplyBleed, true, [1, 1, 1]),
    ],
};

pub const T_BO_02: ItemDefinition = ItemDefinition {
    id: b"T-BO-02\0",
    name: "Reaper Pick",
    item_type: ItemType::Tool,
    tag: ItemTag::Blood,
    rarity: Rarity::Rare,
    effects: &[
        // +2/3/4 ATK
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [2, 3, 4],
        ),
        // On Hit (once/turn): apply 1 Bleed
        EffectDefinition::new(TriggerType::OnHit, EffectType::ApplyBleed, true, [1, 1, 1]),
        // If enemy is Wounded, apply +1 Bleed
        EffectDefinition::with_condition(
            TriggerType::OnHit,
            EffectType::ApplyBleed,
            true,
            [1, 1, 1],
            Condition::EnemyWounded,
        ),
    ],
};

pub const G_BO_01: ItemDefinition = ItemDefinition {
    id: b"G-BO-01\0",
    name: "Last Breath Sigil",
    item_type: ItemType::Gear,
    tag: ItemTag::Blood,
    rarity: Rarity::Common,
    effects: &[
        // One use: first time you would die in battle, prevent it and heal 2/3/4 HP
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::PreventDeath,
            false,
            [2, 3, 4], // Heal amount when death is prevented
        ),
    ],
};

pub const G_BO_02: ItemDefinition = ItemDefinition {
    id: b"G-BO-02\0",
    name: "Bloodletting Fang",
    item_type: ItemType::Gear,
    tag: ItemTag::Blood,
    rarity: Rarity::Rare, // GDD says Rare
    effects: &[
        // Your attacks deal +1/2/3 damage to Bleeding enemies
        // Implemented as OnHit bonus damage with EnemyHasBleed condition
        EffectDefinition::with_condition(
            TriggerType::OnHit,
            EffectType::DealDamage,
            false, // Each hit deals extra damage
            [1, 2, 3],
            Condition::EnemyHasStatus(StatusType::Bleed),
        ),
    ],
};

pub const G_BO_03: ItemDefinition = ItemDefinition {
    id: b"G-BO-03\0",
    name: "Leech Wraps",
    item_type: ItemType::Gear,
    tag: ItemTag::Blood,
    rarity: Rarity::Rare,
    effects: &[
        // When enemy takes Bleed damage: heal 1/2/3 HP (once/turn)
        EffectDefinition::new(
            TriggerType::OnEnemyBleedDamage,
            EffectType::Heal,
            true, // once per turn
            [1, 2, 3],
        ),
    ],
};

pub const G_BO_04: ItemDefinition = ItemDefinition {
    id: b"G-BO-04\0",
    name: "Blood Chalice",
    item_type: ItemType::Gear,
    tag: ItemTag::Blood,
    rarity: Rarity::Rare,
    effects: &[
        // Victory: heal 3/5/7 HP
        EffectDefinition::new(TriggerType::Victory, EffectType::Heal, false, [3, 5, 7]),
    ],
};

pub const G_BO_05: ItemDefinition = ItemDefinition {
    id: b"G-BO-05\0",
    name: "Hemorrhage Hook",
    item_type: ItemType::Gear,
    tag: ItemTag::Blood,
    rarity: Rarity::Heroic, // GDD says Heroic
    effects: &[
        // Wounded: apply 2/3/4 Bleed
        EffectDefinition::new(
            TriggerType::Wounded,
            EffectType::ApplyBleed,
            true,
            [2, 3, 4],
        ),
    ],
};

pub const G_BO_06: ItemDefinition = ItemDefinition {
    id: b"G-BO-06\0",
    name: "Execution Emblem",
    item_type: ItemType::Gear,
    tag: ItemTag::Blood,
    rarity: Rarity::Heroic, // GDD says Heroic
    effects: &[
        // If enemy is Wounded, your first strike each turn deals +2/3/4 damage
        EffectDefinition::with_condition(
            TriggerType::OnHit,
            EffectType::DealDamage,
            true, // once per turn (first strike only)
            [2, 3, 4],
            Condition::EnemyWounded,
        ),
    ],
};

pub const G_BO_07: ItemDefinition = ItemDefinition {
    id: b"G-BO-07\0",
    name: "Gore Mantle",
    item_type: ItemType::Gear,
    tag: ItemTag::Blood,
    rarity: Rarity::Rare,
    effects: &[
        // First time you become Wounded in battle: gain 4/6/8 Armor
        EffectDefinition::new(
            TriggerType::FirstTimeWounded,
            EffectType::GainArmor,
            false,
            [4, 6, 8],
        ),
    ],
};

pub const G_BO_08: ItemDefinition = ItemDefinition {
    id: b"G-BO-08\0",
    name: "Vampiric Tooth",
    item_type: ItemType::Gear,
    tag: ItemTag::Blood,
    rarity: Rarity::Mythic, // GDD says Mythic
    effects: &[
        // Your first hit each turn vs a Bleeding enemy heals more (cap 5/7/9)
        EffectDefinition::with_condition(
            TriggerType::OnHit,
            EffectType::Heal,
            true, // once per turn (first hit only)
            [5, 7, 9],
            Condition::EnemyHasStatus(StatusType::Bleed),
        ),
        // Self-enable bleed on hit
        EffectDefinition::new(TriggerType::OnHit, EffectType::ApplyBleed, true, [1, 1, 1]),
    ],
};

// =============================================================================
// TEMPO Tag Items (T-TE-01, T-TE-02, G-TE-01 through G-TE-08)
// Theme: SPD, Turn 1 effects
// =============================================================================

pub const T_TE_01: ItemDefinition = ItemDefinition {
    id: b"T-TE-01\0",
    name: "Quickpick",
    item_type: ItemType::Tool,
    tag: ItemTag::Tempo,
    rarity: Rarity::Common,
    effects: &[
        // +1/2/3 ATK
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [1, 2, 3],
        ),
        // +2/3/4 SPD
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainSpd,
            false,
            [2, 3, 4],
        ),
    ],
};

pub const T_TE_02: ItemDefinition = ItemDefinition {
    id: b"T-TE-02\0",
    name: "Chrono Rapier",
    item_type: ItemType::Tool,
    tag: ItemTag::Tempo,
    rarity: Rarity::Mythic,
    effects: &[
        // +2/3/4 ATK
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [2, 3, 4],
        ),
        // +3/4/5 SPD
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainSpd,
            false,
            [3, 4, 5],
        ),
        // If you act first on Turn 1, gain +3/4/5 ATK (this battle)
        EffectDefinition::new(
            TriggerType::FirstTurnIfFaster,
            EffectType::GainAtk,
            false,
            [3, 4, 5],
        ),
    ],
};

pub const G_TE_01: ItemDefinition = ItemDefinition {
    id: b"G-TE-01\0",
    name: "Wind-Up Spring",
    item_type: ItemType::Gear,
    tag: ItemTag::Tempo,
    rarity: Rarity::Common,
    effects: &[
        // Turn 1: gain +1/2/3 SPD (this battle)
        EffectDefinition::new(
            TriggerType::FirstTurn,
            EffectType::GainSpd,
            false,
            [1, 2, 3],
        ),
        // Turn 1: gain +2/3/4 ATK (this battle)
        EffectDefinition::new(
            TriggerType::FirstTurn,
            EffectType::GainAtk,
            false,
            [2, 3, 4],
        ),
    ],
};

pub const G_TE_02: ItemDefinition = ItemDefinition {
    id: b"G-TE-02\0",
    name: "Ambush Charm",
    item_type: ItemType::Gear,
    tag: ItemTag::Tempo,
    rarity: Rarity::Rare, // GDD says Rare
    effects: &[
        // If you act first on Turn 1, your first strike deals +3/5/7 damage
        EffectDefinition::new(
            TriggerType::FirstTurnIfFaster,
            EffectType::DealDamage,
            true, // once per turn (first strike only)
            [3, 5, 7],
        ),
    ],
};

pub const G_TE_03: ItemDefinition = ItemDefinition {
    id: b"G-TE-03\0",
    name: "Counterweight Buckle",
    item_type: ItemType::Gear,
    tag: ItemTag::Tempo,
    rarity: Rarity::Rare,
    effects: &[
        // If enemy acts first on Turn 1, gain 5/7/9 Armor before damage
        EffectDefinition::new(
            TriggerType::FirstTurnIfSlower,
            EffectType::GainArmor,
            false,
            [5, 7, 9],
        ),
    ],
};

pub const G_TE_04: ItemDefinition = ItemDefinition {
    id: b"G-TE-04\0",
    name: "Hourglass Charge",
    item_type: ItemType::Gear,
    tag: ItemTag::Tempo,
    rarity: Rarity::Rare,
    effects: &[
        // Turn 5: gain +2/3/4 ATK (this battle)
        EffectDefinition::new(
            TriggerType::TurnN { turn: 5 },
            EffectType::GainAtk,
            false,
            [2, 3, 4],
        ),
        // Turn 5: gain +1 SPD (this battle)
        EffectDefinition::new(
            TriggerType::TurnN { turn: 5 },
            EffectType::GainSpd,
            false,
            [1, 1, 1],
        ),
    ],
};

pub const G_TE_05: ItemDefinition = ItemDefinition {
    id: b"G-TE-05\0",
    name: "Initiative Lens",
    item_type: ItemType::Gear,
    tag: ItemTag::Tempo,
    rarity: Rarity::Rare,
    effects: &[
        // +1/2/3 SPD
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainSpd,
            false,
            [1, 2, 3],
        ),
        // Battle Start: if your SPD > enemy SPD, gain 3/5/7 Armor
        EffectDefinition::with_condition(
            TriggerType::BattleStart,
            EffectType::GainArmor,
            false,
            [3, 5, 7],
            Condition::SpdGreaterThanEnemySpd,
        ),
    ],
};

pub const G_TE_06: ItemDefinition = ItemDefinition {
    id: b"G-TE-06\0",
    name: "Backstep Buckle",
    item_type: ItemType::Gear,
    tag: ItemTag::Tempo,
    rarity: Rarity::Rare, // GDD says Rare
    effects: &[
        // If enemy acts first on Turn 1, gain 4/6/8 Armor
        EffectDefinition::new(
            TriggerType::FirstTurnIfSlower,
            EffectType::GainArmor,
            false,
            [4, 6, 8],
        ),
        // If enemy acts first on Turn 1, your first strike deals +3/5/7 damage
        EffectDefinition::new(
            TriggerType::FirstTurnIfSlower,
            EffectType::DealDamage,
            true, // once per turn (first strike only)
            [3, 5, 7],
        ),
    ],
};

pub const G_TE_07: ItemDefinition = ItemDefinition {
    id: b"G-TE-07\0",
    name: "Tempo Battery",
    item_type: ItemType::Gear,
    tag: ItemTag::Tempo,
    rarity: Rarity::Heroic,
    effects: &[
        // EveryOtherTurn: +1/2/3 SPD per GDD
        EffectDefinition::new(
            TriggerType::EveryOtherTurn,
            EffectType::GainSpd,
            false,
            [1, 2, 3],
        ),
    ],
};

pub const G_TE_08: ItemDefinition = ItemDefinition {
    id: b"G-TE-08\0",
    name: "Second Wind Clock",
    item_type: ItemType::Gear,
    tag: ItemTag::Tempo,
    rarity: Rarity::Heroic, // GDD says Heroic, not Mythic
    effects: &[
        // Turn 5: heal 4/6/8 HP
        EffectDefinition::new(
            TriggerType::TurnN { turn: 5 },
            EffectType::Heal,
            false,
            [4, 6, 8],
        ),
        // Turn 5: gain +1 SPD (this battle)
        EffectDefinition::new(
            TriggerType::TurnN { turn: 5 },
            EffectType::GainSpd,
            false,
            [1, 1, 1],
        ),
    ],
};

// =============================================================================
// All Items Array (80 items total)
// =============================================================================

/// Core 80-item registry (16 tools + 64 gear).
pub const ITEMS: &[ItemDefinition] = &[
    // STONE (10)
    T_ST_01, T_ST_02, G_ST_01, G_ST_02, G_ST_03, G_ST_04, G_ST_05, G_ST_06, G_ST_07, G_ST_08,
    // SCOUT (10)
    T_SC_01, T_SC_02, G_SC_01, G_SC_02, G_SC_03, G_SC_04, G_SC_05, G_SC_06, G_SC_07, G_SC_08,
    // GREED (10)
    T_GR_01, T_GR_02, G_GR_01, G_GR_02, G_GR_03, G_GR_04, G_GR_05, G_GR_06, G_GR_07, G_GR_08,
    // BLAST (10)
    T_BL_01, T_BL_02, G_BL_01, G_BL_02, G_BL_03, G_BL_04, G_BL_05, G_BL_06, G_BL_07, G_BL_08,
    // FROST (10)
    T_FR_01, T_FR_02, G_FR_01, G_FR_02, G_FR_03, G_FR_04, G_FR_05, G_FR_06, G_FR_07, G_FR_08,
    // RUST (10)
    T_RU_01, T_RU_02, G_RU_01, G_RU_02, G_RU_03, G_RU_04, G_RU_05, G_RU_06, G_RU_07, G_RU_08,
    // BLOOD (10)
    T_BO_01, T_BO_02, G_BO_01, G_BO_02, G_BO_03, G_BO_04, G_BO_05, G_BO_06, G_BO_07, G_BO_08,
    // TEMPO (10)
    T_TE_01, T_TE_02, G_TE_01, G_TE_02, G_TE_03, G_TE_04, G_TE_05, G_TE_06, G_TE_07, G_TE_08,
];

// =============================================================================
// Query Functions
// =============================================================================

/// Get an item by its ID
/// Handles the special case of BASIC_PICKAXE (T-XX-00) which is not in the main ITEMS array
pub fn get_item(id: &[u8; 8]) -> Option<&'static ItemDefinition> {
    // Special case: starter tool is not in ITEMS array (excluded from offers/fusion)
    if id == b"T-XX-00\0" {
        return Some(&BASIC_PICKAXE);
    }
    ITEMS.iter().find(|i| i.id == id)
}

/// Get all items with a specific tag
pub fn get_items_by_tag(tag: ItemTag) -> Vec<&'static ItemDefinition> {
    ITEMS.iter().filter(|i| i.tag == tag).collect()
}

/// Get all items with a specific rarity
pub fn get_items_by_rarity(rarity: Rarity) -> Vec<&'static ItemDefinition> {
    ITEMS.iter().filter(|i| i.rarity == rarity).collect()
}

/// Get all items of a specific type
pub fn get_items_by_type(item_type: ItemType) -> Vec<&'static ItemDefinition> {
    ITEMS.iter().filter(|i| i.item_type == item_type).collect()
}

fn rarity_from_index(rarity_index: usize) -> Option<Rarity> {
    match rarity_index {
        0 => Some(Rarity::Common),
        1 => Some(Rarity::Rare),
        2 => Some(Rarity::Heroic),
        3 => Some(Rarity::Mythic),
        _ => None,
    }
}

fn tag_from_index(tag_index: usize) -> Option<ItemTag> {
    match tag_index {
        0 => Some(ItemTag::Stone),
        1 => Some(ItemTag::Scout),
        2 => Some(ItemTag::Greed),
        3 => Some(ItemTag::Blast),
        4 => Some(ItemTag::Frost),
        5 => Some(ItemTag::Rust),
        6 => Some(ItemTag::Blood),
        7 => Some(ItemTag::Tempo),
        _ => None,
    }
}

/// Get gear item indices for a given rarity and tag.
pub fn gear_by_rarity_tag(rarity_index: usize, tag_index: usize) -> Vec<usize> {
    let Some(rarity) = rarity_from_index(rarity_index) else {
        return Vec::new();
    };
    let Some(tag) = tag_from_index(tag_index) else {
        return Vec::new();
    };

    ITEMS
        .iter()
        .enumerate()
        .filter_map(|(idx, item)| {
            (item.item_type == ItemType::Gear && item.rarity == rarity && item.tag == tag)
                .then_some(idx)
        })
        .collect()
}

/// Get tool item indices for a given rarity and tag.
pub fn tool_by_rarity_tag(rarity_index: usize, tag_index: usize) -> Vec<usize> {
    let Some(rarity) = rarity_from_index(rarity_index) else {
        return Vec::new();
    };
    let Some(tag) = tag_from_index(tag_index) else {
        return Vec::new();
    };

    ITEMS
        .iter()
        .enumerate()
        .filter_map(|(idx, item)| {
            (item.item_type == ItemType::Tool && item.rarity == rarity && item.tag == tag)
                .then_some(idx)
        })
        .collect()
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_total_item_count() {
        assert_eq!(ITEMS.len(), 80, "Should have exactly 80 items");
    }

    #[test]
    fn test_get_item_by_id() {
        let item = get_item(b"T-ST-01\0");
        assert!(item.is_some());
        let item = item.unwrap();
        assert_eq!(item.name, "Bulwark Shovel");
        assert_eq!(item.item_type, ItemType::Tool);
        assert_eq!(item.tag, ItemTag::Stone);
        assert_eq!(item.rarity, Rarity::Common);
    }

    #[test]
    fn test_get_invalid_item() {
        let item = get_item(b"X-XX-99\0");
        assert!(item.is_none());
    }

    #[test]
    fn test_basic_pickaxe_returned_by_get_item() {
        // Verify the starter tool can be retrieved via get_item
        let item = get_item(b"T-XX-00\0");
        assert!(
            item.is_some(),
            "BASIC_PICKAXE should be returned by get_item"
        );

        let pickaxe = item.unwrap();
        assert_eq!(pickaxe.name, "Basic Pickaxe");
        assert_eq!(pickaxe.item_type, ItemType::Tool);
        assert_eq!(pickaxe.tag, ItemTag::None);
        assert_eq!(pickaxe.rarity, Rarity::Common);

        // Verify effects
        assert_eq!(pickaxe.effects.len(), 1);
        assert_eq!(pickaxe.effects[0].trigger, TriggerType::BattleStart);
        assert_eq!(pickaxe.effects[0].effect_type, EffectType::GainAtk);
        assert_eq!(pickaxe.effects[0].values, [1, 1, 1]); // Always +1 ATK
    }

    #[test]
    fn test_basic_pickaxe_not_in_items_array() {
        // Verify BASIC_PICKAXE is NOT in the main ITEMS array
        // (to ensure it's excluded from offer generation/fusion)
        assert!(!ITEMS.iter().any(|i| i.id == b"T-XX-00\0"));
    }

    #[test]
    fn test_items_by_tag_count() {
        for (tag, expected) in [
            (ItemTag::Stone, 10usize),
            (ItemTag::Scout, 10),
            (ItemTag::Greed, 10),
            (ItemTag::Blast, 10),
            (ItemTag::Frost, 10),
            (ItemTag::Rust, 10),
            (ItemTag::Blood, 10),
            (ItemTag::Tempo, 10),
        ] {
            let items = get_items_by_tag(tag);
            assert_eq!(
                items.len(),
                expected,
                "Unexpected item count for tag {:?}",
                tag
            );
        }
    }

    #[test]
    fn test_tools_vs_gear_count() {
        let tools = get_items_by_type(ItemType::Tool);
        let gear = get_items_by_type(ItemType::Gear);
        assert_eq!(tools.len(), 16, "Should have 16 tool items");
        assert_eq!(gear.len(), 64, "Should have 64 gear items");
    }

    #[test]
    fn test_rarity_distribution() {
        // Rarity distribution after revised balance alignment
        // Note: BASIC_PICKAXE (Common) is NOT in ITEMS array
        // Changes from GDD alignment:
        // - G-FR-02: Common -> Rare (Frostguard Buckler)
        // - G-FR-08: Mythic -> Heroic (Deep Freeze Charm)
        // - G-TE-08: Mythic -> Heroic (Second Wind Clock)
        // - G-GR-05: Rare -> Common (Emerald Shard)
        // - G-GR-06: Heroic -> Common (Ruby Shard)
        // - G-GR-07: Heroic -> Common (Sapphire Shard)
        // - G-GR-08: Mythic -> Common (Citrine Shard)
        // - G-ST-08: Mythic -> Heroic (Stone Sigil)
        // - G-BO-02: Common -> Rare (Bloodletting Fang)
        // - G-BO-07: Heroic -> Rare (Gore Mantle)
        // - G-RU-08: Mythic -> Common (Salvage Clamp)
        // - T-GR-02: Rare -> Heroic (Gemfinder Staff)
        // - G-GR-04: Rare -> Heroic (Royal Bracer)
        // - G-BL-02: Common -> Rare (Blast Suit)
        // - G-BL-05: Rare -> Heroic (Bomb Satchel)
        // - G-BL-06: Heroic -> Rare (Kindling Charge)
        // - G-ST-06: Heroic -> Rare (Shrapnel Talisman)
        let common = get_items_by_rarity(Rarity::Common);
        let rare = get_items_by_rarity(Rarity::Rare);
        let heroic = get_items_by_rarity(Rarity::Heroic);
        let mythic = get_items_by_rarity(Rarity::Mythic);

        let total = common.len() + rare.len() + heroic.len() + mythic.len();
        assert_eq!(total, 80, "Total should be 80 items");
        assert_eq!(common.len(), 25, "Should have 25 Common items");
        assert_eq!(rare.len(), 31, "Should have 31 Rare items");
        assert_eq!(heroic.len(), 16, "Should have 16 Heroic items");
        assert_eq!(mythic.len(), 8, "Should have 8 Mythic items");
    }

    #[test]
    fn test_frost_items_specifically() {
        let frost_items = get_items_by_tag(ItemTag::Frost);
        assert_eq!(frost_items.len(), 10);

        // Check that Rime Pike exists
        let rime_pike = get_item(b"T-FR-01\0");
        assert!(rime_pike.is_some());
        let rp = rime_pike.unwrap();
        assert_eq!(rp.name, "Rime Pike");
        assert_eq!(rp.effects.len(), 3);

        // Check effect values - Chill is flat 1 per GDD (not tier-scaling)
        let on_hit_effect = rp
            .effects
            .iter()
            .find(|e| e.effect_type == EffectType::ApplyChill);
        assert!(on_hit_effect.is_some());
        let effect = on_hit_effect.unwrap();
        assert_eq!(effect.effect_type, EffectType::ApplyChill);
        assert_eq!(effect.values, [1, 1, 1]); // Flat 1 Chill per GDD
    }

    #[test]
    fn test_item_index_consistency() {
        let tags = [
            ItemTag::Stone,
            ItemTag::Scout,
            ItemTag::Greed,
            ItemTag::Blast,
            ItemTag::Frost,
            ItemTag::Rust,
            ItemTag::Blood,
            ItemTag::Tempo,
        ];
        let rarities = [Rarity::Common, Rarity::Rare, Rarity::Heroic, Rarity::Mythic];

        for (ri, &rarity) in rarities.iter().enumerate() {
            for (ti, &tag) in tags.iter().enumerate() {
                // Verify gear index
                let gear_indices = gear_by_rarity_tag(ri, ti);
                for &idx in &gear_indices {
                    let item = &ITEMS[idx];
                    assert_eq!(
                        item.item_type,
                        ItemType::Gear,
                        "Gear index {idx} has wrong type"
                    );
                    assert_eq!(item.rarity, rarity, "Gear index {idx} has wrong rarity");
                    assert_eq!(item.tag, tag, "Gear index {idx} has wrong tag");
                }

                // Verify count matches brute-force
                let brute_count = ITEMS
                    .iter()
                    .filter(|i| i.item_type == ItemType::Gear && i.rarity == rarity && i.tag == tag)
                    .count();
                assert_eq!(
                    gear_indices.len(),
                    brute_count,
                    "Gear index count mismatch for rarity={ri} tag={ti}"
                );

                // Verify tool index
                let tool_indices = tool_by_rarity_tag(ri, ti);
                for &idx in &tool_indices {
                    let item = &ITEMS[idx];
                    assert_eq!(
                        item.item_type,
                        ItemType::Tool,
                        "Tool index {idx} has wrong type"
                    );
                    assert_eq!(item.rarity, rarity, "Tool index {idx} has wrong rarity");
                    assert_eq!(item.tag, tag, "Tool index {idx} has wrong tag");
                }

                let brute_count = ITEMS
                    .iter()
                    .filter(|i| i.item_type == ItemType::Tool && i.rarity == rarity && i.tag == tag)
                    .count();
                assert_eq!(
                    tool_indices.len(),
                    brute_count,
                    "Tool index count mismatch for rarity={ri} tag={ti}"
                );
            }
        }
    }
}
