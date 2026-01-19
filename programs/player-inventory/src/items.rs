//! Item definitions for the player-inventory program.
//!
//! Contains all 80 item definitions as compile-time constants.
//! 16 Tools (2 per tag) + 64 Gear (8 per tag) across 8 tags.

use crate::state::{EffectDefinition, EffectType, ItemTag, ItemType, Rarity, TriggerType};

/// Static item definition (compile-time constant)
#[derive(Clone, Copy, Debug)]
pub struct ItemDefinition {
    /// Unique ID (e.g., "T-ST-01")
    pub id: &'static [u8; 8],
    /// Display name (e.g., "Bulwark Shovel")
    pub name: &'static str,
    /// Tool or Gear
    pub item_type: ItemType,
    /// One of 8 tags
    pub tag: ItemTag,
    /// Common, Rare, Heroic, or Mythic
    pub rarity: Rarity,
    /// Combat effects with tier-scaled values
    pub effects: &'static [EffectDefinition],
}

// =============================================================================
// STONE Tag Items (T-ST-01, T-ST-02, G-ST-01 through G-ST-08)
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
    name: "Rampart Mattock",
    item_type: ItemType::Tool,
    tag: ItemTag::Stone,
    rarity: Rarity::Rare,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [2, 3, 4],
        ),
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainArmor,
            false,
            [3, 5, 7],
        ),
        EffectDefinition::new(
            TriggerType::OnHit,
            EffectType::ApplyShrapnel,
            true,
            [1, 1, 2],
        ),
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
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainArmor,
            false,
            [2, 4, 6],
        ),
        EffectDefinition::new(TriggerType::BattleStart, EffectType::Heal, false, [2, 3, 4]),
    ],
};

pub const G_ST_03: ItemDefinition = ItemDefinition {
    id: b"G-ST-03\0",
    name: "Spiked Bracers",
    item_type: ItemType::Gear,
    tag: ItemTag::Stone,
    rarity: Rarity::Rare,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::ApplyShrapnel,
        false,
        [2, 3, 4],
    )],
};

pub const G_ST_04: ItemDefinition = ItemDefinition {
    id: b"G-ST-04\0",
    name: "Stone Shield",
    item_type: ItemType::Gear,
    tag: ItemTag::Stone,
    rarity: Rarity::Rare,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::GainArmor,
        false,
        [5, 8, 11],
    )],
};

pub const G_ST_05: ItemDefinition = ItemDefinition {
    id: b"G-ST-05\0",
    name: "Granite Greaves",
    item_type: ItemType::Gear,
    tag: ItemTag::Stone,
    rarity: Rarity::Rare,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainArmor,
            false,
            [4, 6, 8],
        ),
        EffectDefinition::new(TriggerType::Wounded, EffectType::GainArmor, true, [3, 5, 7]),
    ],
};

pub const G_ST_06: ItemDefinition = ItemDefinition {
    id: b"G-ST-06\0",
    name: "Shrapnel Collar",
    item_type: ItemType::Gear,
    tag: ItemTag::Stone,
    rarity: Rarity::Heroic,
    effects: &[EffectDefinition::new(
        TriggerType::TurnStart,
        EffectType::ApplyShrapnel,
        false,
        [1, 2, 3],
    )],
};

pub const G_ST_07: ItemDefinition = ItemDefinition {
    id: b"G-ST-07\0",
    name: "Bastion Plate",
    item_type: ItemType::Gear,
    tag: ItemTag::Stone,
    rarity: Rarity::Heroic,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::GainArmor,
        false,
        [8, 12, 16],
    )],
};

pub const G_ST_08: ItemDefinition = ItemDefinition {
    id: b"G-ST-08\0",
    name: "Adamant Core",
    item_type: ItemType::Gear,
    tag: ItemTag::Stone,
    rarity: Rarity::Mythic,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainArmor,
            false,
            [6, 10, 14],
        ),
        EffectDefinition::new(
            TriggerType::TurnStart,
            EffectType::GainArmor,
            false,
            [2, 3, 4],
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
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [1, 2, 3],
        ),
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainDig,
            false,
            [1, 2, 3],
        ),
    ],
};

pub const T_SC_02: ItemDefinition = ItemDefinition {
    id: b"T-SC-02\0",
    name: "Swift Spade",
    item_type: ItemType::Tool,
    tag: ItemTag::Scout,
    rarity: Rarity::Rare,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [2, 3, 4],
        ),
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainDig,
            false,
            [2, 3, 4],
        ),
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainSpd,
            false,
            [1, 1, 2],
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
    name: "Explorer Goggles",
    item_type: ItemType::Gear,
    tag: ItemTag::Scout,
    rarity: Rarity::Common,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainDig,
            false,
            [1, 2, 3],
        ),
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainSpd,
            false,
            [1, 1, 2],
        ),
    ],
};

pub const G_SC_03: ItemDefinition = ItemDefinition {
    id: b"G-SC-03\0",
    name: "Pathfinder Cloak",
    item_type: ItemType::Gear,
    tag: ItemTag::Scout,
    rarity: Rarity::Rare,
    effects: &[EffectDefinition::new(
        TriggerType::FirstTurn,
        EffectType::GainDig,
        false,
        [3, 5, 7],
    )],
};

pub const G_SC_04: ItemDefinition = ItemDefinition {
    id: b"G-SC-04\0",
    name: "Sprint Greaves",
    item_type: ItemType::Gear,
    tag: ItemTag::Scout,
    rarity: Rarity::Rare,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::GainSpd,
        false,
        [2, 3, 4],
    )],
};

pub const G_SC_05: ItemDefinition = ItemDefinition {
    id: b"G-SC-05\0",
    name: "Double Strike Gloves",
    item_type: ItemType::Gear,
    tag: ItemTag::Scout,
    rarity: Rarity::Rare,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainDig,
            false,
            [2, 3, 4],
        ),
        EffectDefinition::new(TriggerType::OnHit, EffectType::DealDamage, true, [1, 2, 3]),
    ],
};

pub const G_SC_06: ItemDefinition = ItemDefinition {
    id: b"G-SC-06\0",
    name: "Tunnel Runner Belt",
    item_type: ItemType::Gear,
    tag: ItemTag::Scout,
    rarity: Rarity::Heroic,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::GainDig,
        false,
        [4, 6, 8],
    )],
};

pub const G_SC_07: ItemDefinition = ItemDefinition {
    id: b"G-SC-07\0",
    name: "Rapid Excavator",
    item_type: ItemType::Gear,
    tag: ItemTag::Scout,
    rarity: Rarity::Heroic,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainDig,
            false,
            [3, 5, 7],
        ),
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainSpd,
            false,
            [2, 3, 4],
        ),
    ],
};

pub const G_SC_08: ItemDefinition = ItemDefinition {
    id: b"G-SC-08\0",
    name: "Phantom Pickaxe",
    item_type: ItemType::Gear,
    tag: ItemTag::Scout,
    rarity: Rarity::Mythic,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainDig,
            false,
            [5, 8, 11],
        ),
        EffectDefinition::new(
            TriggerType::FirstTurn,
            EffectType::GainSpd,
            false,
            [3, 4, 5],
        ),
    ],
};

// =============================================================================
// GREED Tag Items (T-GR-01, T-GR-02, G-GR-01 through G-GR-08)
// Theme: Gold generation, shard effects
// =============================================================================

pub const T_GR_01: ItemDefinition = ItemDefinition {
    id: b"T-GR-01\0",
    name: "Gold Digger",
    item_type: ItemType::Tool,
    tag: ItemTag::Greed,
    rarity: Rarity::Common,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [1, 2, 3],
        ),
        EffectDefinition::new(TriggerType::OnHit, EffectType::GainGold, true, [1, 2, 3]),
    ],
};

pub const T_GR_02: ItemDefinition = ItemDefinition {
    id: b"T-GR-02\0",
    name: "Fortune Finder",
    item_type: ItemType::Tool,
    tag: ItemTag::Greed,
    rarity: Rarity::Rare,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [2, 3, 4],
        ),
        EffectDefinition::new(TriggerType::OnHit, EffectType::GainGold, false, [2, 3, 4]),
    ],
};

pub const G_GR_01: ItemDefinition = ItemDefinition {
    id: b"G-GR-01\0",
    name: "Coin Pouch",
    item_type: ItemType::Gear,
    tag: ItemTag::Greed,
    rarity: Rarity::Common,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::GainGold,
        false,
        [3, 5, 7],
    )],
};

pub const G_GR_02: ItemDefinition = ItemDefinition {
    id: b"G-GR-02\0",
    name: "Lucky Charm",
    item_type: ItemType::Gear,
    tag: ItemTag::Greed,
    rarity: Rarity::Common,
    effects: &[EffectDefinition::new(
        TriggerType::TurnStart,
        EffectType::GainGold,
        false,
        [1, 1, 2],
    )],
};

pub const G_GR_03: ItemDefinition = ItemDefinition {
    id: b"G-GR-03\0",
    name: "Treasure Hunter Belt",
    item_type: ItemType::Gear,
    tag: ItemTag::Greed,
    rarity: Rarity::Rare,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::GainGold,
        false,
        [5, 8, 11],
    )],
};

pub const G_GR_04: ItemDefinition = ItemDefinition {
    id: b"G-GR-04\0",
    name: "Gold Converter",
    item_type: ItemType::Gear,
    tag: ItemTag::Greed,
    rarity: Rarity::Rare,
    effects: &[
        EffectDefinition::new(
            TriggerType::TurnStart,
            EffectType::GainArmor,
            false,
            [1, 2, 3],
        ),
        EffectDefinition::new(
            TriggerType::TurnStart,
            EffectType::GainGold,
            false,
            [1, 2, 3],
        ),
    ],
};

pub const G_GR_05: ItemDefinition = ItemDefinition {
    id: b"G-GR-05\0",
    name: "Shard Collector",
    item_type: ItemType::Gear,
    tag: ItemTag::Greed,
    rarity: Rarity::Rare,
    effects: &[EffectDefinition::new(
        TriggerType::EveryOtherTurn,
        EffectType::GainGold,
        false,
        [4, 6, 8],
    )],
};

pub const G_GR_06: ItemDefinition = ItemDefinition {
    id: b"G-GR-06\0",
    name: "Wealth Amplifier",
    item_type: ItemType::Gear,
    tag: ItemTag::Greed,
    rarity: Rarity::Heroic,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::GainGold,
        false,
        [8, 12, 16],
    )],
};

pub const G_GR_07: ItemDefinition = ItemDefinition {
    id: b"G-GR-07\0",
    name: "Shard Matrix",
    item_type: ItemType::Gear,
    tag: ItemTag::Greed,
    rarity: Rarity::Heroic,
    effects: &[EffectDefinition::new(
        TriggerType::EveryOtherTurn,
        EffectType::GainGold,
        false,
        [6, 9, 12],
    )],
};

pub const G_GR_08: ItemDefinition = ItemDefinition {
    id: b"G-GR-08\0",
    name: "Midas Touch",
    item_type: ItemType::Gear,
    tag: ItemTag::Greed,
    rarity: Rarity::Mythic,
    effects: &[
        EffectDefinition::new(TriggerType::OnHit, EffectType::GainGold, false, [3, 5, 7]),
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainGold,
            false,
            [5, 8, 11],
        ),
    ],
};

// =============================================================================
// BLAST Tag Items (T-BL-01, T-BL-02, G-BL-01 through G-BL-08)
// Theme: Countdown bombs, non-weapon damage
// =============================================================================

pub const T_BL_01: ItemDefinition = ItemDefinition {
    id: b"T-BL-01\0",
    name: "Blast Pick",
    item_type: ItemType::Tool,
    tag: ItemTag::Blast,
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
            EffectType::ApplyBomb,
            false,
            [1, 1, 2],
        ),
    ],
};

pub const T_BL_02: ItemDefinition = ItemDefinition {
    id: b"T-BL-02\0",
    name: "Demolition Hammer",
    item_type: ItemType::Tool,
    tag: ItemTag::Blast,
    rarity: Rarity::Rare,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [2, 3, 4],
        ),
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::ApplyBomb,
            false,
            [2, 3, 4],
        ),
    ],
};

pub const G_BL_01: ItemDefinition = ItemDefinition {
    id: b"G-BL-01\0",
    name: "Fuse Box",
    item_type: ItemType::Gear,
    tag: ItemTag::Blast,
    rarity: Rarity::Common,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::ApplyBomb,
        false,
        [1, 2, 3],
    )],
};

pub const G_BL_02: ItemDefinition = ItemDefinition {
    id: b"G-BL-02\0",
    name: "Powder Keg",
    item_type: ItemType::Gear,
    tag: ItemTag::Blast,
    rarity: Rarity::Common,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::DealNonWeaponDamage,
        false,
        [3, 5, 7],
    )],
};

pub const G_BL_03: ItemDefinition = ItemDefinition {
    id: b"G-BL-03\0",
    name: "Detonator Belt",
    item_type: ItemType::Gear,
    tag: ItemTag::Blast,
    rarity: Rarity::Rare,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::ApplyBomb,
        false,
        [2, 3, 4],
    )],
};

pub const G_BL_04: ItemDefinition = ItemDefinition {
    id: b"G-BL-04\0",
    name: "Shockwave Gauntlets",
    item_type: ItemType::Gear,
    tag: ItemTag::Blast,
    rarity: Rarity::Rare,
    effects: &[EffectDefinition::new(
        TriggerType::OnHit,
        EffectType::DealNonWeaponDamage,
        true,
        [2, 4, 6],
    )],
};

pub const G_BL_05: ItemDefinition = ItemDefinition {
    id: b"G-BL-05\0",
    name: "Bomb Amplifier",
    item_type: ItemType::Gear,
    tag: ItemTag::Blast,
    rarity: Rarity::Rare,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::ApplyBomb,
            false,
            [1, 2, 3],
        ),
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::DealNonWeaponDamage,
            false,
            [2, 3, 4],
        ),
    ],
};

pub const G_BL_06: ItemDefinition = ItemDefinition {
    id: b"G-BL-06\0",
    name: "Chain Reaction Core",
    item_type: ItemType::Gear,
    tag: ItemTag::Blast,
    rarity: Rarity::Heroic,
    effects: &[EffectDefinition::new(
        TriggerType::TurnStart,
        EffectType::ApplyBomb,
        false,
        [1, 1, 2],
    )],
};

pub const G_BL_07: ItemDefinition = ItemDefinition {
    id: b"G-BL-07\0",
    name: "Volatile Container",
    item_type: ItemType::Gear,
    tag: ItemTag::Blast,
    rarity: Rarity::Heroic,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::ApplyBomb,
        false,
        [4, 6, 8],
    )],
};

pub const G_BL_08: ItemDefinition = ItemDefinition {
    id: b"G-BL-08\0",
    name: "Nuclear Core",
    item_type: ItemType::Gear,
    tag: ItemTag::Blast,
    rarity: Rarity::Mythic,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::ApplyBomb,
            false,
            [3, 5, 7],
        ),
        EffectDefinition::new(
            TriggerType::TurnStart,
            EffectType::DealNonWeaponDamage,
            false,
            [2, 3, 4],
        ),
    ],
};

// =============================================================================
// FROST Tag Items (T-FR-01, T-FR-02, G-FR-01 through G-FR-08)
// Theme: Chill, SPD manipulation
// =============================================================================

pub const T_FR_01: ItemDefinition = ItemDefinition {
    id: b"T-FR-01\0",
    name: "Rime Pike",
    item_type: ItemType::Tool,
    tag: ItemTag::Frost,
    rarity: Rarity::Common,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [2, 3, 4],
        ),
        EffectDefinition::new(TriggerType::OnHit, EffectType::ApplyChill, true, [1, 2, 3]),
    ],
};

pub const T_FR_02: ItemDefinition = ItemDefinition {
    id: b"T-FR-02\0",
    name: "Glacier Breaker",
    item_type: ItemType::Tool,
    tag: ItemTag::Frost,
    rarity: Rarity::Rare,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [2, 3, 4],
        ),
        EffectDefinition::new(TriggerType::OnHit, EffectType::ApplyChill, false, [2, 3, 4]),
    ],
};

pub const G_FR_01: ItemDefinition = ItemDefinition {
    id: b"G-FR-01\0",
    name: "Frost Amulet",
    item_type: ItemType::Gear,
    tag: ItemTag::Frost,
    rarity: Rarity::Common,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::ApplyChill,
        false,
        [2, 3, 4],
    )],
};

pub const G_FR_02: ItemDefinition = ItemDefinition {
    id: b"G-FR-02\0",
    name: "Icy Gauntlets",
    item_type: ItemType::Gear,
    tag: ItemTag::Frost,
    rarity: Rarity::Common,
    effects: &[EffectDefinition::new(
        TriggerType::OnHit,
        EffectType::ApplyChill,
        true,
        [1, 2, 3],
    )],
};

pub const G_FR_03: ItemDefinition = ItemDefinition {
    id: b"G-FR-03\0",
    name: "Blizzard Cloak",
    item_type: ItemType::Gear,
    tag: ItemTag::Frost,
    rarity: Rarity::Rare,
    effects: &[EffectDefinition::new(
        TriggerType::TurnStart,
        EffectType::ApplyChill,
        false,
        [1, 1, 2],
    )],
};

pub const G_FR_04: ItemDefinition = ItemDefinition {
    id: b"G-FR-04\0",
    name: "Permafrost Boots",
    item_type: ItemType::Gear,
    tag: ItemTag::Frost,
    rarity: Rarity::Rare,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::ApplyChill,
        false,
        [3, 5, 7],
    )],
};

pub const G_FR_05: ItemDefinition = ItemDefinition {
    id: b"G-FR-05\0",
    name: "Frozen Heart",
    item_type: ItemType::Gear,
    tag: ItemTag::Frost,
    rarity: Rarity::Rare,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::ApplyChill,
            false,
            [2, 4, 6],
        ),
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainSpd,
            false,
            [1, 2, 3],
        ),
    ],
};

pub const G_FR_06: ItemDefinition = ItemDefinition {
    id: b"G-FR-06\0",
    name: "Cryo Chamber",
    item_type: ItemType::Gear,
    tag: ItemTag::Frost,
    rarity: Rarity::Heroic,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::ApplyChill,
        false,
        [5, 8, 11],
    )],
};

pub const G_FR_07: ItemDefinition = ItemDefinition {
    id: b"G-FR-07\0",
    name: "Avalanche Trigger",
    item_type: ItemType::Gear,
    tag: ItemTag::Frost,
    rarity: Rarity::Heroic,
    effects: &[EffectDefinition::new(
        TriggerType::TurnStart,
        EffectType::ApplyChill,
        false,
        [2, 3, 4],
    )],
};

pub const G_FR_08: ItemDefinition = ItemDefinition {
    id: b"G-FR-08\0",
    name: "Absolute Zero",
    item_type: ItemType::Gear,
    tag: ItemTag::Frost,
    rarity: Rarity::Mythic,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::ApplyChill,
            false,
            [6, 10, 14],
        ),
        EffectDefinition::new(
            TriggerType::FirstTurn,
            EffectType::GainSpd,
            false,
            [3, 4, 5],
        ),
    ],
};

// =============================================================================
// RUST Tag Items (T-RU-01, T-RU-02, G-RU-01 through G-RU-08)
// Theme: Armor destruction
// =============================================================================

pub const T_RU_01: ItemDefinition = ItemDefinition {
    id: b"T-RU-01\0",
    name: "Corrosive Pick",
    item_type: ItemType::Tool,
    tag: ItemTag::Rust,
    rarity: Rarity::Common,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [2, 3, 4],
        ),
        EffectDefinition::new(TriggerType::OnHit, EffectType::ApplyRust, true, [1, 2, 3]),
    ],
};

pub const T_RU_02: ItemDefinition = ItemDefinition {
    id: b"T-RU-02\0",
    name: "Acid Excavator",
    item_type: ItemType::Tool,
    tag: ItemTag::Rust,
    rarity: Rarity::Rare,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [2, 3, 4],
        ),
        EffectDefinition::new(TriggerType::OnHit, EffectType::ApplyRust, false, [2, 3, 4]),
    ],
};

pub const G_RU_01: ItemDefinition = ItemDefinition {
    id: b"G-RU-01\0",
    name: "Rust Vial",
    item_type: ItemType::Gear,
    tag: ItemTag::Rust,
    rarity: Rarity::Common,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::ApplyRust,
        false,
        [2, 3, 4],
    )],
};

pub const G_RU_02: ItemDefinition = ItemDefinition {
    id: b"G-RU-02\0",
    name: "Corrosion Spreader",
    item_type: ItemType::Gear,
    tag: ItemTag::Rust,
    rarity: Rarity::Common,
    effects: &[EffectDefinition::new(
        TriggerType::OnHit,
        EffectType::ApplyRust,
        true,
        [1, 2, 3],
    )],
};

pub const G_RU_03: ItemDefinition = ItemDefinition {
    id: b"G-RU-03\0",
    name: "Oxidation Chamber",
    item_type: ItemType::Gear,
    tag: ItemTag::Rust,
    rarity: Rarity::Rare,
    effects: &[EffectDefinition::new(
        TriggerType::TurnStart,
        EffectType::ApplyRust,
        false,
        [1, 1, 2],
    )],
};

pub const G_RU_04: ItemDefinition = ItemDefinition {
    id: b"G-RU-04\0",
    name: "Decay Amplifier",
    item_type: ItemType::Gear,
    tag: ItemTag::Rust,
    rarity: Rarity::Rare,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::ApplyRust,
        false,
        [3, 5, 7],
    )],
};

pub const G_RU_05: ItemDefinition = ItemDefinition {
    id: b"G-RU-05\0",
    name: "Entropy Engine",
    item_type: ItemType::Gear,
    tag: ItemTag::Rust,
    rarity: Rarity::Rare,
    effects: &[EffectDefinition::new(
        TriggerType::Exposed,
        EffectType::DealDamage,
        true,
        [3, 5, 7],
    )],
};

pub const G_RU_06: ItemDefinition = ItemDefinition {
    id: b"G-RU-06\0",
    name: "Tetanus Injector",
    item_type: ItemType::Gear,
    tag: ItemTag::Rust,
    rarity: Rarity::Heroic,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::ApplyRust,
            false,
            [4, 6, 8],
        ),
        EffectDefinition::new(TriggerType::OnHit, EffectType::ApplyRust, true, [1, 2, 3]),
    ],
};

pub const G_RU_07: ItemDefinition = ItemDefinition {
    id: b"G-RU-07\0",
    name: "Disintegration Field",
    item_type: ItemType::Gear,
    tag: ItemTag::Rust,
    rarity: Rarity::Heroic,
    effects: &[EffectDefinition::new(
        TriggerType::TurnStart,
        EffectType::ApplyRust,
        false,
        [2, 3, 4],
    )],
};

pub const G_RU_08: ItemDefinition = ItemDefinition {
    id: b"G-RU-08\0",
    name: "Total Corrosion",
    item_type: ItemType::Gear,
    tag: ItemTag::Rust,
    rarity: Rarity::Mythic,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::ApplyRust,
            false,
            [5, 8, 11],
        ),
        EffectDefinition::new(
            TriggerType::TurnStart,
            EffectType::RemoveArmor,
            false,
            [1, 2, 3],
        ),
    ],
};

// =============================================================================
// BLOOD Tag Items (T-BO-01, T-BO-02, G-BO-01 through G-BO-08)
// Theme: Bleed, sustain
// =============================================================================

pub const T_BO_01: ItemDefinition = ItemDefinition {
    id: b"T-BO-01\0",
    name: "Bloodletter",
    item_type: ItemType::Tool,
    tag: ItemTag::Blood,
    rarity: Rarity::Common,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [2, 3, 4],
        ),
        EffectDefinition::new(TriggerType::OnHit, EffectType::ApplyBleed, true, [1, 2, 3]),
    ],
};

pub const T_BO_02: ItemDefinition = ItemDefinition {
    id: b"T-BO-02\0",
    name: "Crimson Excavator",
    item_type: ItemType::Tool,
    tag: ItemTag::Blood,
    rarity: Rarity::Rare,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [2, 3, 4],
        ),
        EffectDefinition::new(TriggerType::OnHit, EffectType::ApplyBleed, false, [2, 3, 4]),
    ],
};

pub const G_BO_01: ItemDefinition = ItemDefinition {
    id: b"G-BO-01\0",
    name: "Blood Vial",
    item_type: ItemType::Gear,
    tag: ItemTag::Blood,
    rarity: Rarity::Common,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::ApplyBleed,
        false,
        [2, 3, 4],
    )],
};

pub const G_BO_02: ItemDefinition = ItemDefinition {
    id: b"G-BO-02\0",
    name: "Leech Ring",
    item_type: ItemType::Gear,
    tag: ItemTag::Blood,
    rarity: Rarity::Common,
    effects: &[EffectDefinition::new(
        TriggerType::OnHit,
        EffectType::Heal,
        true,
        [1, 2, 3],
    )],
};

pub const G_BO_03: ItemDefinition = ItemDefinition {
    id: b"G-BO-03\0",
    name: "Hemorrhage Gloves",
    item_type: ItemType::Gear,
    tag: ItemTag::Blood,
    rarity: Rarity::Rare,
    effects: &[EffectDefinition::new(
        TriggerType::OnHit,
        EffectType::ApplyBleed,
        true,
        [2, 3, 4],
    )],
};

pub const G_BO_04: ItemDefinition = ItemDefinition {
    id: b"G-BO-04\0",
    name: "Vitality Siphon",
    item_type: ItemType::Gear,
    tag: ItemTag::Blood,
    rarity: Rarity::Rare,
    effects: &[EffectDefinition::new(
        TriggerType::TurnStart,
        EffectType::Heal,
        false,
        [1, 2, 3],
    )],
};

pub const G_BO_05: ItemDefinition = ItemDefinition {
    id: b"G-BO-05\0",
    name: "Blood Surge",
    item_type: ItemType::Gear,
    tag: ItemTag::Blood,
    rarity: Rarity::Rare,
    effects: &[EffectDefinition::new(
        TriggerType::Wounded,
        EffectType::GainAtk,
        true,
        [2, 3, 4],
    )],
};

pub const G_BO_06: ItemDefinition = ItemDefinition {
    id: b"G-BO-06\0",
    name: "Crimson Tide",
    item_type: ItemType::Gear,
    tag: ItemTag::Blood,
    rarity: Rarity::Heroic,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::ApplyBleed,
        false,
        [4, 6, 8],
    )],
};

pub const G_BO_07: ItemDefinition = ItemDefinition {
    id: b"G-BO-07\0",
    name: "Life Drain Aura",
    item_type: ItemType::Gear,
    tag: ItemTag::Blood,
    rarity: Rarity::Heroic,
    effects: &[
        EffectDefinition::new(TriggerType::TurnStart, EffectType::Heal, false, [2, 3, 4]),
        EffectDefinition::new(
            TriggerType::TurnStart,
            EffectType::ApplyBleed,
            false,
            [1, 2, 3],
        ),
    ],
};

pub const G_BO_08: ItemDefinition = ItemDefinition {
    id: b"G-BO-08\0",
    name: "Exsanguinate",
    item_type: ItemType::Gear,
    tag: ItemTag::Blood,
    rarity: Rarity::Mythic,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::ApplyBleed,
            false,
            [5, 8, 11],
        ),
        EffectDefinition::new(TriggerType::OnHit, EffectType::Heal, false, [2, 3, 4]),
    ],
};

// =============================================================================
// TEMPO Tag Items (T-TE-01, T-TE-02, G-TE-01 through G-TE-08)
// Theme: SPD, Turn 1 effects
// =============================================================================

pub const T_TE_01: ItemDefinition = ItemDefinition {
    id: b"T-TE-01\0",
    name: "Quick Strike",
    item_type: ItemType::Tool,
    tag: ItemTag::Tempo,
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
            EffectType::GainSpd,
            false,
            [2, 3, 4],
        ),
    ],
};

pub const T_TE_02: ItemDefinition = ItemDefinition {
    id: b"T-TE-02\0",
    name: "Tempo Hammer",
    item_type: ItemType::Tool,
    tag: ItemTag::Tempo,
    rarity: Rarity::Rare,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainAtk,
            false,
            [2, 3, 4],
        ),
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainSpd,
            false,
            [3, 4, 5],
        ),
        EffectDefinition::new(
            TriggerType::FirstTurn,
            EffectType::DealDamage,
            true,
            [2, 4, 6],
        ),
    ],
};

pub const G_TE_01: ItemDefinition = ItemDefinition {
    id: b"G-TE-01\0",
    name: "Speed Charm",
    item_type: ItemType::Gear,
    tag: ItemTag::Tempo,
    rarity: Rarity::Common,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::GainSpd,
        false,
        [2, 3, 4],
    )],
};

pub const G_TE_02: ItemDefinition = ItemDefinition {
    id: b"G-TE-02\0",
    name: "First Strike Gloves",
    item_type: ItemType::Gear,
    tag: ItemTag::Tempo,
    rarity: Rarity::Common,
    effects: &[EffectDefinition::new(
        TriggerType::FirstTurn,
        EffectType::DealDamage,
        false,
        [3, 5, 7],
    )],
};

pub const G_TE_03: ItemDefinition = ItemDefinition {
    id: b"G-TE-03\0",
    name: "Initiative Badge",
    item_type: ItemType::Gear,
    tag: ItemTag::Tempo,
    rarity: Rarity::Rare,
    effects: &[
        EffectDefinition::new(
            TriggerType::FirstTurn,
            EffectType::GainAtk,
            false,
            [2, 3, 4],
        ),
        EffectDefinition::new(
            TriggerType::FirstTurn,
            EffectType::GainSpd,
            false,
            [2, 3, 4],
        ),
    ],
};

pub const G_TE_04: ItemDefinition = ItemDefinition {
    id: b"G-TE-04\0",
    name: "Momentum Belt",
    item_type: ItemType::Gear,
    tag: ItemTag::Tempo,
    rarity: Rarity::Rare,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::GainSpd,
        false,
        [3, 5, 7],
    )],
};

pub const G_TE_05: ItemDefinition = ItemDefinition {
    id: b"G-TE-05\0",
    name: "Haste Amulet",
    item_type: ItemType::Gear,
    tag: ItemTag::Tempo,
    rarity: Rarity::Rare,
    effects: &[EffectDefinition::new(
        TriggerType::TurnStart,
        EffectType::GainSpd,
        false,
        [1, 1, 2],
    )],
};

pub const G_TE_06: ItemDefinition = ItemDefinition {
    id: b"G-TE-06\0",
    name: "Overdrive Core",
    item_type: ItemType::Gear,
    tag: ItemTag::Tempo,
    rarity: Rarity::Heroic,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::GainSpd,
        false,
        [4, 6, 8],
    )],
};

pub const G_TE_07: ItemDefinition = ItemDefinition {
    id: b"G-TE-07\0",
    name: "Alpha Strike Module",
    item_type: ItemType::Gear,
    tag: ItemTag::Tempo,
    rarity: Rarity::Heroic,
    effects: &[
        EffectDefinition::new(
            TriggerType::FirstTurn,
            EffectType::DealDamage,
            false,
            [5, 8, 11],
        ),
        EffectDefinition::new(
            TriggerType::FirstTurn,
            EffectType::GainSpd,
            false,
            [2, 3, 4],
        ),
    ],
};

pub const G_TE_08: ItemDefinition = ItemDefinition {
    id: b"G-TE-08\0",
    name: "Temporal Accelerator",
    item_type: ItemType::Gear,
    tag: ItemTag::Tempo,
    rarity: Rarity::Mythic,
    effects: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainSpd,
            false,
            [5, 8, 11],
        ),
        EffectDefinition::new(
            TriggerType::FirstTurn,
            EffectType::GainAtk,
            false,
            [3, 5, 7],
        ),
    ],
};

// =============================================================================
// All Items Array (80 items total)
// =============================================================================

/// All 80 item definitions as a compile-time constant array
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
pub fn get_item(id: &[u8; 8]) -> Option<&'static ItemDefinition> {
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
    fn test_items_by_tag_count() {
        for tag in [
            ItemTag::Stone,
            ItemTag::Scout,
            ItemTag::Greed,
            ItemTag::Blast,
            ItemTag::Frost,
            ItemTag::Rust,
            ItemTag::Blood,
            ItemTag::Tempo,
        ] {
            let items = get_items_by_tag(tag);
            assert_eq!(items.len(), 10, "Each tag should have exactly 10 items");
        }
    }

    #[test]
    fn test_tools_vs_gear_count() {
        let tools = get_items_by_type(ItemType::Tool);
        let gear = get_items_by_type(ItemType::Gear);
        assert_eq!(tools.len(), 16, "Should have 16 tools (2 per tag)");
        assert_eq!(gear.len(), 64, "Should have 64 gear items (8 per tag)");
    }

    #[test]
    fn test_rarity_distribution() {
        // 2 Tools per tag: 1 Common, 1 Rare
        // 8 Gear per tag: 2 Common, 3 Rare, 2 Heroic, 1 Mythic
        // Per tag totals: 3 Common, 4 Rare, 2 Heroic, 1 Mythic = 10
        // x8 tags: 24 Common, 32 Rare, 16 Heroic, 8 Mythic = 80
        let common = get_items_by_rarity(Rarity::Common);
        let rare = get_items_by_rarity(Rarity::Rare);
        let heroic = get_items_by_rarity(Rarity::Heroic);
        let mythic = get_items_by_rarity(Rarity::Mythic);

        assert_eq!(common.len(), 24, "Should have 24 Common items");
        assert_eq!(rare.len(), 32, "Should have 32 Rare items");
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
        assert_eq!(rp.effects.len(), 2);

        // Check effect values for tier scaling
        let on_hit_effect = rp.effects.iter().find(|e| e.trigger == TriggerType::OnHit);
        assert!(on_hit_effect.is_some());
        let effect = on_hit_effect.unwrap();
        assert_eq!(effect.effect_type, EffectType::ApplyChill);
        assert_eq!(effect.values, [1, 2, 3]); // Tier I=1, II=2, III=3
    }
}
