use crate::{traits::*, Biome, BossStats, EffectType, ItemEffect, ItemTag, Week};
use combat_system::state::{AnnotatedItemEffect, CombatSourceKind, CombatSourceRef};

/// Static definition of a boss
#[derive(Clone, Debug)]
pub struct BossDefinition {
    pub id: [u8; 12],
    pub name: &'static str,
    pub emoji: [u8; 4],
    pub biome: Biome,
    pub week: Week,
    pub boss_index: u8,
    pub base_stats: BossStats,
    pub weakness_tags: [ItemTag; 2],
    pub traits: &'static [BossTrait],
    /// Base strikes per turn (1 for most, 3 for Broodmother, etc.)
    pub base_strikes: u8,
}

impl BossDefinition {
    pub fn get_base_strikes(&self) -> u8 {
        // Check if there's a BattleStart GainStrikes trait
        let extra_strikes: i16 = self
            .traits
            .iter()
            .filter(|t| {
                matches!(t.trigger, crate::TriggerType::BattleStart)
                    && matches!(t.effect_type, crate::EffectType::GainStrikes)
            })
            .map(|t| t.value)
            .sum();
        (self.base_strikes as i16 + extra_strikes).max(1) as u8
    }
}

/// Helper to create a 12-byte ID from a string
const fn id_from_str(s: &str) -> [u8; 12] {
    let bytes = s.as_bytes();
    let mut id = [0u8; 12];
    let mut i = 0;
    while i < bytes.len() && i < 12 {
        id[i] = bytes[i];
        i += 1;
    }
    id
}

/// Helper to create a 4-byte emoji array
const fn emoji_from_bytes(a: u8, b: u8, c: u8, d: u8) -> [u8; 4] {
    [a, b, c, d]
}

// ============================================================================
// Week 1 Biome A Bosses (5)
// ============================================================================

pub static BROODMOTHER_A: BossDefinition = BossDefinition {
    id: id_from_str("B-A-W1-01"),
    name: "The Broodmother",
    emoji: emoji_from_bytes(0xF0, 0x9F, 0x95, 0xB7), // spider
    biome: Biome::A,
    week: Week::One,
    boss_index: 0,
    base_stats: BossStats {
        hp: 24,
        atk: 2,
        arm: 1,
        spd: 2,
        dig: 1,
    },
    weakness_tags: [ItemTag::Stone, ItemTag::Frost],
    traits: BROODMOTHER_TRAITS,
    base_strikes: 1,
};

pub static OBSIDIAN_GOLEM_A: BossDefinition = BossDefinition {
    id: id_from_str("B-A-W1-02"),
    name: "Obsidian Golem",
    emoji: emoji_from_bytes(0xF0, 0x9F, 0xAA, 0xA8), // rock
    biome: Biome::A,
    week: Week::One,
    boss_index: 1,
    base_stats: BossStats {
        hp: 28,
        atk: 2,
        arm: 8,
        spd: 0,
        dig: 3,
    },
    weakness_tags: [ItemTag::Rust, ItemTag::Blast],
    traits: OBSIDIAN_GOLEM_TRAITS,
    base_strikes: 1,
};

pub static GAS_ANOMALY_A: BossDefinition = BossDefinition {
    id: id_from_str("B-A-W1-03"),
    name: "Gas Anomaly",
    emoji: emoji_from_bytes(0xF0, 0x9F, 0x92, 0xA8), // dizzy/gas
    biome: Biome::A,
    week: Week::One,
    boss_index: 2,
    base_stats: BossStats {
        hp: 26,
        atk: 2,
        arm: 0,
        spd: 1,
        dig: 2,
    },
    weakness_tags: [ItemTag::Blood, ItemTag::Tempo],
    traits: GAS_ANOMALY_TRAITS,
    base_strikes: 1,
};

pub static MAD_MINER_A: BossDefinition = BossDefinition {
    id: id_from_str("B-A-W1-04"),
    name: "Mad Miner",
    emoji: emoji_from_bytes(0xE2, 0x9B, 0x8F, 0x00), // pick
    biome: Biome::A,
    week: Week::One,
    boss_index: 3,
    base_stats: BossStats {
        hp: 26,
        atk: 2,
        arm: 3,
        spd: 2,
        dig: 3,
    },
    weakness_tags: [ItemTag::Scout, ItemTag::Greed],
    traits: MAD_MINER_TRAITS,
    base_strikes: 1,
};

pub static SHARD_COLOSSUS_A: BossDefinition = BossDefinition {
    id: id_from_str("B-A-W1-05"),
    name: "Shard Colossus",
    emoji: emoji_from_bytes(0xF0, 0x9F, 0x92, 0x8E), // gem
    biome: Biome::A,
    week: Week::One,
    boss_index: 4,
    base_stats: BossStats {
        hp: 26,
        atk: 2,
        arm: 3,
        spd: 1,
        dig: 2,
    },
    weakness_tags: [ItemTag::Stone, ItemTag::Blood],
    traits: SHARD_COLOSSUS_TRAITS,
    base_strikes: 1,
};

// ============================================================================
// Week 2 Biome A Bosses (5)
// ============================================================================

pub static DRILL_SERGEANT_A: BossDefinition = BossDefinition {
    id: id_from_str("B-A-W2-01"),
    name: "Drill Sergeant",
    emoji: emoji_from_bytes(0xF0, 0x9F, 0xAA, 0x96), // drill/mechanical
    biome: Biome::A,
    week: Week::Two,
    boss_index: 0,
    base_stats: BossStats {
        hp: 34,
        atk: 2,
        arm: 6,
        spd: 2,
        dig: 3,
    },
    weakness_tags: [ItemTag::Frost, ItemTag::Tempo],
    traits: DRILL_SERGEANT_TRAITS,
    base_strikes: 1,
};

pub static CRYSTAL_MIMIC_A: BossDefinition = BossDefinition {
    id: id_from_str("B-A-W2-02"),
    name: "Crystal Mimic",
    emoji: emoji_from_bytes(0xF0, 0x9F, 0x92, 0x8E), // gem/crystal
    biome: Biome::A,
    week: Week::Two,
    boss_index: 1,
    base_stats: BossStats {
        hp: 36,
        atk: 3,
        arm: 5,
        spd: 2,
        dig: 2,
    },
    weakness_tags: [ItemTag::Blast, ItemTag::Scout],
    traits: CRYSTAL_MIMIC_TRAITS,
    base_strikes: 1,
};

pub static RUST_REGENT_A: BossDefinition = BossDefinition {
    id: id_from_str("B-A-W2-03"),
    name: "Rust Regent",
    emoji: emoji_from_bytes(0xF0, 0x9F, 0xA4, 0xB4), // rust/brown
    biome: Biome::A,
    week: Week::Two,
    boss_index: 2,
    base_stats: BossStats {
        hp: 36,
        atk: 2,
        arm: 5,
        spd: 2,
        dig: 3,
    },
    weakness_tags: [ItemTag::Blood, ItemTag::Tempo],
    traits: RUST_REGENT_TRAITS,
    base_strikes: 1,
};

pub static POWDER_KEG_BARON_A: BossDefinition = BossDefinition {
    id: id_from_str("B-A-W2-04"),
    name: "Powder Keg Baron",
    emoji: emoji_from_bytes(0xF0, 0x9F, 0x92, 0xA3), // bomb
    biome: Biome::A,
    week: Week::Two,
    boss_index: 3,
    base_stats: BossStats {
        hp: 32,
        atk: 2,
        arm: 4,
        spd: 2,
        dig: 2,
    },
    weakness_tags: [ItemTag::Stone, ItemTag::Frost],
    traits: POWDER_KEG_BARON_TRAITS,
    base_strikes: 1,
};

pub static GREEDKEEPER_A: BossDefinition = BossDefinition {
    id: id_from_str("B-A-W2-05"),
    name: "Greedkeeper",
    emoji: emoji_from_bytes(0xF0, 0x9F, 0x92, 0xB0), // money bag
    biome: Biome::A,
    week: Week::Two,
    boss_index: 4,
    base_stats: BossStats {
        hp: 38,
        atk: 2,
        arm: 6,
        spd: 1,
        dig: 2,
    },
    weakness_tags: [ItemTag::Greed, ItemTag::Rust],
    traits: GREEDKEEPER_TRAITS,
    base_strikes: 1,
};

// ============================================================================
// Week 3 Biome A Final Bosses (2)
// ============================================================================

pub static ELDRITCH_MOLE_A: BossDefinition = BossDefinition {
    id: id_from_str("B-A-W3-01"),
    name: "The Eldritch Mole",
    emoji: emoji_from_bytes(0xF0, 0x9F, 0xA6, 0x94), // hedgehog (mole-like)
    biome: Biome::A,
    week: Week::Three,
    boss_index: 0,
    base_stats: BossStats {
        hp: 50,
        atk: 4,
        arm: 8,
        spd: 3,
        dig: 4,
    },
    weakness_tags: [ItemTag::Rust, ItemTag::Tempo],
    traits: ELDRITCH_MOLE_TRAITS,
    base_strikes: 1,
};

pub static GILDED_DEVOURER_A: BossDefinition = BossDefinition {
    id: id_from_str("B-A-W3-02"),
    name: "The Gilded Devourer",
    emoji: emoji_from_bytes(0xF0, 0x9F, 0x90, 0x89), // dragon
    biome: Biome::A,
    week: Week::Three,
    boss_index: 1,
    base_stats: BossStats {
        hp: 46,
        atk: 3,
        arm: 6,
        spd: 2,
        dig: 3,
    },
    weakness_tags: [ItemTag::Greed, ItemTag::Blood],
    traits: GILDED_DEVOURER_TRAITS,
    base_strikes: 1,
};

// ============================================================================
// Week 1 Biome B Bosses (5) - Same archetypes with +1 SPD (final SPD capped at 3)
// ============================================================================

pub static BROODMOTHER_B: BossDefinition = BossDefinition {
    id: id_from_str("B-B-W1-01"),
    name: "The Broodmother",
    emoji: emoji_from_bytes(0xF0, 0x9F, 0x95, 0xB7),
    biome: Biome::B,
    week: Week::One,
    boss_index: 0,
    base_stats: BossStats {
        hp: 24,
        atk: 2,
        arm: 1,
        spd: 3,
        dig: 1,
    },
    weakness_tags: [ItemTag::Stone, ItemTag::Frost],
    traits: BROODMOTHER_TRAITS,
    base_strikes: 1,
};

pub static OBSIDIAN_GOLEM_B: BossDefinition = BossDefinition {
    id: id_from_str("B-B-W1-02"),
    name: "Obsidian Golem",
    emoji: emoji_from_bytes(0xF0, 0x9F, 0xAA, 0xA8),
    biome: Biome::B,
    week: Week::One,
    boss_index: 1,
    base_stats: BossStats {
        hp: 28,
        atk: 2,
        arm: 8,
        spd: 1,
        dig: 3,
    },
    weakness_tags: [ItemTag::Rust, ItemTag::Blast],
    traits: OBSIDIAN_GOLEM_TRAITS,
    base_strikes: 1,
};

pub static GAS_ANOMALY_B: BossDefinition = BossDefinition {
    id: id_from_str("B-B-W1-03"),
    name: "Gas Anomaly",
    emoji: emoji_from_bytes(0xF0, 0x9F, 0x92, 0xA8),
    biome: Biome::B,
    week: Week::One,
    boss_index: 2,
    base_stats: BossStats {
        hp: 26,
        atk: 2,
        arm: 0,
        spd: 2,
        dig: 2,
    },
    weakness_tags: [ItemTag::Blood, ItemTag::Tempo],
    traits: GAS_ANOMALY_TRAITS,
    base_strikes: 1,
};

pub static MAD_MINER_B: BossDefinition = BossDefinition {
    id: id_from_str("B-B-W1-04"),
    name: "Mad Miner",
    emoji: emoji_from_bytes(0xE2, 0x9B, 0x8F, 0x00),
    biome: Biome::B,
    week: Week::One,
    boss_index: 3,
    base_stats: BossStats {
        hp: 26,
        atk: 2,
        arm: 3,
        spd: 3,
        dig: 3,
    },
    weakness_tags: [ItemTag::Scout, ItemTag::Greed],
    traits: MAD_MINER_TRAITS,
    base_strikes: 1,
};

pub static SHARD_COLOSSUS_B: BossDefinition = BossDefinition {
    id: id_from_str("B-B-W1-05"),
    name: "Shard Colossus",
    emoji: emoji_from_bytes(0xF0, 0x9F, 0x92, 0x8E),
    biome: Biome::B,
    week: Week::One,
    boss_index: 4,
    base_stats: BossStats {
        hp: 26,
        atk: 2,
        arm: 3,
        spd: 2,
        dig: 2,
    },
    weakness_tags: [ItemTag::Stone, ItemTag::Blood],
    traits: SHARD_COLOSSUS_TRAITS,
    base_strikes: 1,
};

// ============================================================================
// Week 2 Biome B Bosses (5) - Same archetypes with +1 SPD (final SPD capped at 3)
// ============================================================================

pub static DRILL_SERGEANT_B: BossDefinition = BossDefinition {
    id: id_from_str("B-B-W2-01"),
    name: "Drill Sergeant",
    emoji: emoji_from_bytes(0xF0, 0x9F, 0xAA, 0x96),
    biome: Biome::B,
    week: Week::Two,
    boss_index: 0,
    base_stats: BossStats {
        hp: 34,
        atk: 2,
        arm: 6,
        spd: 3,
        dig: 3,
    },
    weakness_tags: [ItemTag::Frost, ItemTag::Tempo],
    traits: DRILL_SERGEANT_TRAITS,
    base_strikes: 1,
};

pub static CRYSTAL_MIMIC_B: BossDefinition = BossDefinition {
    id: id_from_str("B-B-W2-02"),
    name: "Crystal Mimic",
    emoji: emoji_from_bytes(0xF0, 0x9F, 0x92, 0x8E),
    biome: Biome::B,
    week: Week::Two,
    boss_index: 1,
    base_stats: BossStats {
        hp: 36,
        atk: 3,
        arm: 5,
        spd: 3,
        dig: 2,
    },
    weakness_tags: [ItemTag::Blast, ItemTag::Scout],
    traits: CRYSTAL_MIMIC_TRAITS,
    base_strikes: 1,
};

pub static RUST_REGENT_B: BossDefinition = BossDefinition {
    id: id_from_str("B-B-W2-03"),
    name: "Rust Regent",
    emoji: emoji_from_bytes(0xF0, 0x9F, 0xA4, 0xB4),
    biome: Biome::B,
    week: Week::Two,
    boss_index: 2,
    base_stats: BossStats {
        hp: 36,
        atk: 2,
        arm: 5,
        spd: 3,
        dig: 3,
    },
    weakness_tags: [ItemTag::Blood, ItemTag::Tempo],
    traits: RUST_REGENT_TRAITS,
    base_strikes: 1,
};

pub static POWDER_KEG_BARON_B: BossDefinition = BossDefinition {
    id: id_from_str("B-B-W2-04"),
    name: "Powder Keg Baron",
    emoji: emoji_from_bytes(0xF0, 0x9F, 0x92, 0xA3),
    biome: Biome::B,
    week: Week::Two,
    boss_index: 3,
    base_stats: BossStats {
        hp: 32,
        atk: 2,
        arm: 4,
        spd: 3,
        dig: 2,
    },
    weakness_tags: [ItemTag::Stone, ItemTag::Frost],
    traits: POWDER_KEG_BARON_TRAITS,
    base_strikes: 1,
};

pub static GREEDKEEPER_B: BossDefinition = BossDefinition {
    id: id_from_str("B-B-W2-05"),
    name: "Greedkeeper",
    emoji: emoji_from_bytes(0xF0, 0x9F, 0x92, 0xB0),
    biome: Biome::B,
    week: Week::Two,
    boss_index: 4,
    base_stats: BossStats {
        hp: 38,
        atk: 2,
        arm: 6,
        spd: 2,
        dig: 2,
    },
    weakness_tags: [ItemTag::Greed, ItemTag::Rust],
    traits: GREEDKEEPER_TRAITS,
    base_strikes: 1,
};

// ============================================================================
// Week 3 Biome B Final Bosses (2) - Unique bosses
// ============================================================================

pub static FROSTBOUND_LEVIATHAN_B: BossDefinition = BossDefinition {
    id: id_from_str("B-B-W3-01"),
    name: "The Frostbound Leviathan",
    emoji: emoji_from_bytes(0xF0, 0x9F, 0x90, 0xB3), // whale
    biome: Biome::B,
    week: Week::Three,
    boss_index: 0,
    base_stats: BossStats {
        hp: 52,
        atk: 3,
        arm: 10,
        spd: 2,
        dig: 3,
    },
    weakness_tags: [ItemTag::Tempo, ItemTag::Stone],
    traits: FROSTBOUND_LEVIATHAN_TRAITS,
    base_strikes: 1,
};

pub static RUSTED_CHRONOMANCER_B: BossDefinition = BossDefinition {
    id: id_from_str("B-B-W3-02"),
    name: "The Rusted Chronomancer",
    emoji: emoji_from_bytes(0xE2, 0x8F, 0xB3, 0x00), // hourglass
    biome: Biome::B,
    week: Week::Three,
    boss_index: 1,
    base_stats: BossStats {
        hp: 48,
        atk: 4,
        arm: 6,
        spd: 3,
        dig: 2,
    },
    weakness_tags: [ItemTag::Rust, ItemTag::Blood],
    traits: RUSTED_CHRONOMANCER_TRAITS,
    base_strikes: 1,
};

// ============================================================================
// Boss Registry
// ============================================================================

/// All 24 boss definitions
pub static BOSSES: &[&BossDefinition] = &[
    // Week 1 Biome A (indices 0-4)
    &BROODMOTHER_A,
    &OBSIDIAN_GOLEM_A,
    &GAS_ANOMALY_A,
    &MAD_MINER_A,
    &SHARD_COLOSSUS_A,
    // Week 2 Biome A (indices 5-9)
    &DRILL_SERGEANT_A,
    &CRYSTAL_MIMIC_A,
    &RUST_REGENT_A,
    &POWDER_KEG_BARON_A,
    &GREEDKEEPER_A,
    // Week 3 Biome A (indices 10-11)
    &ELDRITCH_MOLE_A,
    &GILDED_DEVOURER_A,
    // Week 1 Biome B (indices 12-16)
    &BROODMOTHER_B,
    &OBSIDIAN_GOLEM_B,
    &GAS_ANOMALY_B,
    &MAD_MINER_B,
    &SHARD_COLOSSUS_B,
    // Week 2 Biome B (indices 17-21)
    &DRILL_SERGEANT_B,
    &CRYSTAL_MIMIC_B,
    &RUST_REGENT_B,
    &POWDER_KEG_BARON_B,
    &GREEDKEEPER_B,
    // Week 3 Biome B (indices 22-23)
    &FROSTBOUND_LEVIATHAN_B,
    &RUSTED_CHRONOMANCER_B,
];

/// Get boss by ID
pub fn get_boss(id: &[u8; 12]) -> Option<&'static BossDefinition> {
    BOSSES.iter().find(|b| &b.id == id).copied()
}

/// Get all bosses for a specific biome and week
pub fn get_bosses_by_week(biome: Biome, week: Week) -> Vec<&'static BossDefinition> {
    BOSSES
        .iter()
        .filter(|b| b.biome == biome && b.week == week)
        .copied()
        .collect()
}

/// Get boss by biome, week, and index
pub fn get_boss_by_index(biome: Biome, week: Week, index: u8) -> Option<&'static BossDefinition> {
    get_bosses_by_week(biome, week).get(index as usize).copied()
}

/// Get boss weaknesses for a stage and week (internal helper)
pub fn get_boss_weaknesses_internal(stage: u8, week: Week) -> [ItemTag; 2] {
    let boss = crate::selection::select_boss(stage, week);
    boss.weakness_tags
}

/// Convert boss traits to ItemEffect array for combat system
pub fn get_boss_item_effects(boss: &BossDefinition) -> Vec<ItemEffect> {
    const GILDED_DEVOURER_ID: [u8; 12] = [b'B', b'-', b'A', b'-', b'W', b'3', b'-', b'0', b'2', 0, 0, 0];
    boss.traits
        .iter()
        .filter(|trait_def| {
            !matches!(
                trait_def.special,
                SpecialMechanic::Phase { .. } | SpecialMechanic::ModifyOnWounded { .. }
            )
        })
        .filter(|trait_def| {
            !(boss.id == GILDED_DEVOURER_ID
                && matches!(trait_def.effect_type, EffectType::GoldToArmor))
        })
        .map(|t| t.to_item_effect())
        .collect()
}

pub fn get_boss_annotated_item_effects(boss: &BossDefinition) -> Vec<AnnotatedItemEffect> {
    let mut source_id = [0u8; 16];
    source_id[..12].copy_from_slice(&boss.id);
    let source = CombatSourceRef {
        kind: CombatSourceKind::Boss,
        id: source_id,
    };

    get_boss_item_effects(boss)
        .into_iter()
        .map(|effect| AnnotatedItemEffect {
            effect,
            source: Some(source),
        })
        .collect()
}
