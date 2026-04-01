use crate::{EffectType, ItemEffect, TriggerType};
use combat_system::state::Condition as CombatCondition;

/// Conditions for conditional triggers
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {
    PlayerDigLessThan,
    PlayerExposed,
    ActsFirstTurn1,
    ReflectionDepleted,
}

/// Extended mechanics for complex boss traits
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpecialMechanic {
    None,
    Countdown { turns: u8 },
    Reflection { stacks: u8 },
    Phase { threshold: u8 },
    Conditional { condition: Condition },
    GoldInteraction { steal_amount: u16, armor_ratio: u8 },
    ModifyOnWounded { effect_type: EffectType, value: i16 },
}

/// A boss trait/ability that triggers during combat
#[derive(Clone, Copy, Debug)]
pub struct BossTrait {
    pub trigger: TriggerType,
    pub effect_type: EffectType,
    pub value: i16,
    pub once_per_turn: bool,
    pub special: SpecialMechanic,
}

impl BossTrait {
    pub const fn new(trigger: TriggerType, effect_type: EffectType, value: i16) -> Self {
        Self {
            trigger,
            effect_type,
            value,
            once_per_turn: false,
            special: SpecialMechanic::None,
        }
    }

    pub const fn with_once_per_turn(mut self) -> Self {
        self.once_per_turn = true;
        self
    }

    pub const fn with_special(mut self, special: SpecialMechanic) -> Self {
        self.special = special;
        self
    }

    /// Convert to ItemEffect for combat system
    pub fn to_item_effect(&self) -> ItemEffect {
        let (effect_type, value) = match self.special {
            SpecialMechanic::Reflection { stacks } => {
                (EffectType::ApplyReflection, i16::from(stacks))
            }
            _ => (self.effect_type, self.value),
        };

        let condition = match self.special {
            SpecialMechanic::Conditional {
                condition: Condition::PlayerDigLessThan,
            } => CombatCondition::DigGreaterThanEnemyDig,
            SpecialMechanic::Conditional {
                condition: Condition::PlayerExposed,
            } => CombatCondition::EnemyHasNoArmor,
            _ => CombatCondition::None,
        };

        ItemEffect {
            trigger: self.trigger,
            once_per_turn: self.once_per_turn,
            effect_type,
            value,
            condition,
        }
    }
}

// ============================================================================
// Week 1 Biome A Boss Traits
// ============================================================================

/// The Broodmother: 3 strikes, Chill every other turn
pub static BROODMOTHER_TRAITS: &[BossTrait] = &[
    // Swarm Queen: +1 strike at battle start (total 2)
    BossTrait::new(TriggerType::BattleStart, EffectType::GainStrikes, 1),
    // Webbed Strikes: Apply 1 Chill every other turn
    BossTrait::new(TriggerType::EveryOtherTurn, EffectType::ApplyChill, 1),
];

/// Obsidian Golem: +4 ARM/turn, non-weapon removes 2 ARM
pub static OBSIDIAN_GOLEM_TRAITS: &[BossTrait] = &[
    // Hardened Core: +2 ARM at Turn Start
    BossTrait::new(TriggerType::TurnStart, EffectType::GainArmor, 2),
    // Cracked Shell: Taking non-weapon damage removes 2 of its own Armor
    BossTrait::new(
        TriggerType::OnDealNonWeaponDamage,
        EffectType::RemoveOwnArmor,
        2,
    ),
];

/// Gas Anomaly: 2 non-weapon dmg/turn, +1 SPD when Wounded
pub static GAS_ANOMALY_TRAITS: &[BossTrait] = &[
    // Toxic Seep: 1 non-weapon damage per turn
    BossTrait::new(TriggerType::TurnStart, EffectType::DealNonWeaponDamage, 1),
    // Fume Panic: +1 SPD when Wounded
    BossTrait::new(TriggerType::Wounded, EffectType::GainSpd, 1),
];

/// Mad Miner: Exposed if player DIG < boss, +1 strike if Exposed Turn 1
pub static MAD_MINER_TRAITS: &[BossTrait] = &[
    // Undermine: Player becomes Exposed if their DIG < boss DIG
    BossTrait::new(TriggerType::BattleStart, EffectType::RemoveArmor, 0).with_special(
        SpecialMechanic::Conditional {
            condition: Condition::PlayerDigLessThan,
        },
    ),
    // Claim Jump: +1 strike if player Exposed on Turn 1
    BossTrait::new(TriggerType::FirstTurn, EffectType::GainStrikes, 1).with_special(
        SpecialMechanic::Conditional {
            condition: Condition::PlayerExposed,
        },
    ),
];

/// Shard Colossus: +8 Shrapnel start, +4 Shrapnel/2 turns
pub static SHARD_COLOSSUS_TRAITS: &[BossTrait] = &[
    // Prismatic Spines: +4 Shrapnel at Battle Start
    BossTrait::new(TriggerType::BattleStart, EffectType::ApplyShrapnel, 4),
    // Refracting Hide: +2 Shrapnel every 2 turns
    BossTrait::new(TriggerType::EveryOtherTurn, EffectType::ApplyShrapnel, 2),
];

// ============================================================================
// Week 2 Biome A Boss Traits
// ============================================================================

/// Drill Sergeant: +1 ATK +1 SPD/turn (max +4 each), +2 ARM/2 turns
pub static DRILL_SERGEANT_TRAITS: &[BossTrait] = &[
    // Rev Up: +1 ATK at Turn Start (capped at +4 in combat-system)
    BossTrait::new(TriggerType::TurnStart, EffectType::GainAtk, 1),
    // Rev Up: +1 SPD at Turn Start (capped at +4 in combat-system)
    BossTrait::new(TriggerType::TurnStart, EffectType::GainSpd, 1),
    // Formation: +1 ARM every 2 turns
    BossTrait::new(TriggerType::EveryOtherTurn, EffectType::GainArmor, 1),
];

/// Crystal Mimic: 2 reflection stacks, +2 non-weapon taken when depleted
pub static CRYSTAL_MIMIC_TRAITS: &[BossTrait] = &[
    // Prismatic Reflection: 2 stacks, reflects status effects
    BossTrait::new(TriggerType::BattleStart, EffectType::ApplyReflection, 2)
        .with_special(SpecialMechanic::Reflection { stacks: 2 }),
];

/// Rust Regent: 1 Rust/hit, 2 dmg if Exposed at turn start
pub static RUST_REGENT_TRAITS: &[BossTrait] = &[
    // Corroding Edict: Apply 1 Rust on hit
    BossTrait::new(TriggerType::OnHit, EffectType::ApplyRust, 1),
    // Execution Tax: 2 damage if player Exposed at Turn Start
    BossTrait::new(TriggerType::TurnStart, EffectType::DealDamage, 1).with_special(
        SpecialMechanic::Conditional {
            condition: Condition::PlayerExposed,
        },
    ),
];

/// Powder Keg Baron: 3-turn countdown for 10 dmg both, -1 countdown when Wounded
pub static POWDER_KEG_BARON_TRAITS: &[BossTrait] = &[
    // Volatile Countdown: 8 non-weapon damage to enemy after 3 turns
    BossTrait::new(
        TriggerType::Countdown { turns: 3 },
        EffectType::DealNonWeaponDamage,
        8,
    )
    .with_special(SpecialMechanic::Countdown { turns: 3 }),
    // Volatile Countdown: 8 non-weapon damage to self after 3 turns
    BossTrait::new(
        TriggerType::Countdown { turns: 3 },
        EffectType::DealSelfNonWeaponDamage,
        8,
    )
    .with_special(SpecialMechanic::Countdown { turns: 3 }),
    // Short Fuse: -1 countdown when Wounded
    BossTrait::new(TriggerType::Wounded, EffectType::DealDamage, 0).with_special(
        SpecialMechanic::ModifyOnWounded {
            effect_type: EffectType::DealDamage,
            value: 0,
        },
    ),
];

/// Greedkeeper: Steal 16 Gold, ARM = stolen/4 (cap 4)
pub static GREEDKEEPER_TRAITS: &[BossTrait] = &[
    // Toll Collector: Steal 16 Gold at Battle Start
    BossTrait::new(TriggerType::BattleStart, EffectType::StealGold, 16),
    // Gilded Barrier: ARM = stolen gold / 4 (cap 4)
    BossTrait::new(TriggerType::BattleStart, EffectType::GoldToArmor, 4).with_special(
        SpecialMechanic::GoldInteraction {
            steal_amount: 16,
            armor_ratio: 4,
        },
    ),
];

// ============================================================================
// Week 3 Biome A Final Boss Traits
// ============================================================================

/// The Eldritch Mole: Three phases (75%: +6 ARM, 50%: 2 strikes, 25%: 2 Bleed/turn)
pub static ELDRITCH_MOLE_TRAITS: &[BossTrait] = &[
    // Phase 1 (75%): +6 Armor
    BossTrait::new(TriggerType::TurnStart, EffectType::GainArmor, 6)
        .with_special(SpecialMechanic::Phase { threshold: 75 }),
    // Phase 2 (50%): +1 strike (attacks twice)
    BossTrait::new(TriggerType::TurnStart, EffectType::GainStrikes, 1)
        .with_special(SpecialMechanic::Phase { threshold: 50 }),
    // Phase 3 (25%): Apply 2 Bleed per turn
    BossTrait::new(TriggerType::TurnStart, EffectType::ApplyBleed, 2)
        .with_special(SpecialMechanic::Phase { threshold: 25 }),
];

/// The Gilded Devourer: Gold→ARM (+1/3 cap 12), Wounded: 2 Bleed
pub static GILDED_DEVOURER_TRAITS: &[BossTrait] = &[
    // Tax Feast: Gold to ARM conversion at Battle Start
    BossTrait::new(TriggerType::BattleStart, EffectType::GoldToArmor, 3).with_special(
        SpecialMechanic::GoldInteraction {
            steal_amount: 0,
            armor_ratio: 3,
        },
    ),
    // Hunger: Apply 2 Bleed when Wounded
    BossTrait::new(TriggerType::Wounded, EffectType::ApplyBleed, 2),
];

// ============================================================================
// Week 3 Biome B Final Boss Traits
// ============================================================================

/// The Frostbound Leviathan: 2 Chill start, +3 ARM/2 turns, first-time Exposed: clear Chill +2 SPD
pub static FROSTBOUND_LEVIATHAN_TRAITS: &[BossTrait] = &[
    // Whiteout: 2 Chill at Battle Start
    BossTrait::new(TriggerType::BattleStart, EffectType::ApplyChill, 2),
    // Glacial Bulk: +3 ARM every 2 turns
    BossTrait::new(TriggerType::EveryOtherTurn, EffectType::GainArmor, 3),
    // Crack Ice: First time Exposed, clear Chill and +2 SPD
    BossTrait::new(TriggerType::FirstTimeExposed, EffectType::GainSpd, 2),
];

/// The Rusted Chronomancer: 2 strikes T1, 1 Rust/turn, Wounded: 3 Bleed
pub static RUSTED_CHRONOMANCER_TRAITS: &[BossTrait] = &[
    // Time Shear: +1 strike on Turn 1 (total 2)
    BossTrait::new(TriggerType::FirstTurn, EffectType::GainStrikes, 1),
    // Oxidized Future: 1 Rust per turn
    BossTrait::new(TriggerType::TurnStart, EffectType::ApplyRust, 1),
    // Blood Price: 3 Bleed when Wounded
    BossTrait::new(TriggerType::Wounded, EffectType::ApplyBleed, 3),
];

// ============================================================================
// Phase State for Week 3 Finals
// ============================================================================

/// Tracks HP threshold phase activations for Week 3 finals
#[derive(Clone, Copy, Debug, Default)]
pub struct PhaseState {
    pub phase_1_triggered: bool,
    pub phase_2_triggered: bool,
    pub phase_3_triggered: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use combat_system::state::{Condition as CombatCondition, TriggerType as CombatTriggerType};

    #[test]
    fn test_player_dig_less_than_maps_to_owner_dig_greater_than_enemy_dig() {
        let effect = MAD_MINER_TRAITS[0].to_item_effect();
        assert_eq!(effect.condition, CombatCondition::DigGreaterThanEnemyDig);
    }

    #[test]
    fn test_reflection_special_maps_to_apply_reflection() {
        let effect = CRYSTAL_MIMIC_TRAITS[0].to_item_effect();
        assert_eq!(effect.effect_type, EffectType::ApplyReflection);
        assert_eq!(effect.value, 2);
        assert_eq!(effect.trigger, CombatTriggerType::BattleStart);
    }

    #[test]
    fn test_obsidian_golem_uses_non_weapon_trigger_and_removes_own_armor() {
        let effect = OBSIDIAN_GOLEM_TRAITS[1].to_item_effect();
        assert_eq!(effect.trigger, CombatTriggerType::OnDealNonWeaponDamage);
        assert_eq!(effect.effect_type, EffectType::RemoveOwnArmor);
        assert_eq!(effect.value, 2);
    }
}

// ============================================================================
// Act+ Trait Lines (Act C and Act D)
// ============================================================================

/// Act+ trait configuration for a boss archetype.
/// Extra traits are additive — they stack on top of the base traits.
pub struct ActPlusTraits {
    /// Additional trait effects for Act C (act index 2)
    pub act_c: &'static [BossTrait],
    /// Additional trait effects for Act D (act index 3)
    pub act_d: &'static [BossTrait],
}

// --- W1 Bosses ---

/// Broodmother Act+ traits
/// C: Brood Frenzy: Wounded → +1 strike (total 3 with base 2)
/// D: Sticky Webs: Chill also reduces SPD by 1
pub static BROODMOTHER_ACT_PLUS: ActPlusTraits = ActPlusTraits {
    act_c: &[
        // Brood Frenzy: Wounded → gain +1 strike (attacks 3/turn instead of 2)
        BossTrait::new(TriggerType::Wounded, EffectType::GainStrikes, 1),
    ],
    act_d: &[
        // Sticky Webs: Webbed Strikes also reduce SPD by 1
        BossTrait::new(TriggerType::EveryOtherTurn, EffectType::ReduceEnemySpd, 1),
    ],
};

/// Obsidian Golem Act+ traits
/// C: Magma Core: TurnStart if Armor >= 10, deal 1 non-weapon damage
/// D: Obsidian Reform: Every 3rd turn, regain 3 Armor
pub static OBSIDIAN_GOLEM_ACT_PLUS: ActPlusTraits = ActPlusTraits {
    act_c: &[
        // Magma Core: TurnStart, if Armor >= 10, deal 1 non-weapon damage
        BossTrait::new(TriggerType::TurnStart, EffectType::DealNonWeaponDamage, 1).with_special(
            SpecialMechanic::Conditional {
                condition: Condition::ActsFirstTurn1, // Reused as placeholder — actual condition checked in combat
            },
        ),
    ],
    act_d: &[
        // Obsidian Reform: Every 3rd turn, regain 3 Armor
        BossTrait::new(
            TriggerType::Countdown { turns: 3 },
            EffectType::GainArmor,
            3,
        ),
    ],
};

/// Gas Anomaly Act+ traits
/// C: Dense Fumes: Toxic Seep deals +1 more (total 2)
/// D: Panic Spread: Additional SPD gain at 75% HP
pub static GAS_ANOMALY_ACT_PLUS: ActPlusTraits = ActPlusTraits {
    act_c: &[
        // Dense Fumes: +1 non-weapon damage per turn (stacks with base 1 = 2 total)
        BossTrait::new(TriggerType::TurnStart, EffectType::DealNonWeaponDamage, 1),
    ],
    act_d: &[
        // Panic Spread: Gain SPD earlier (Phase 75% threshold, additive with base Wounded)
        BossTrait::new(TriggerType::TurnStart, EffectType::GainSpd, 1)
            .with_special(SpecialMechanic::Phase { threshold: 75 }),
    ],
};

/// Mad Miner Act+ traits
/// C: Tunnel Network: DIG treated as +2 higher
/// D: Claim Stake: if Exposed on Turn 1, steal 5 Gold
pub static MAD_MINER_ACT_PLUS: ActPlusTraits = ActPlusTraits {
    act_c: &[
        // Tunnel Network: +2 DIG for comparisons
        BossTrait::new(TriggerType::BattleStart, EffectType::GainDig, 2),
    ],
    act_d: &[
        // Claim Stake: if player Exposed on Turn 1, steal 5 Gold
        BossTrait::new(TriggerType::FirstTurn, EffectType::StealGold, 5).with_special(
            SpecialMechanic::Conditional {
                condition: Condition::PlayerExposed,
            },
        ),
    ],
};

/// Shard Colossus Act+ traits
/// C: Crystal Lattice: Refracting Hide gains +1 more Shrapnel (3 instead of 2)
/// D: Hardened Spines: While boss has 4+ Shrapnel, gain +2 ARM at turn start
pub static SHARD_COLOSSUS_ACT_PLUS: ActPlusTraits = ActPlusTraits {
    act_c: &[
        // Crystal Lattice: +1 Shrapnel every other turn (stacks with base 2 = 3)
        BossTrait::new(TriggerType::EveryOtherTurn, EffectType::ApplyShrapnel, 1),
    ],
    act_d: &[
        // Hardened Spines: While 4+ Shrapnel, +2 ARM at turn start
        BossTrait::new(TriggerType::TurnStart, EffectType::GainArmor, 2).with_special(
            SpecialMechanic::Conditional {
                condition: Condition::ActsFirstTurn1, // Placeholder for OwnerShrapnelAtLeast(4)
            },
        ),
    ],
};

// --- W2 Bosses ---

/// Drill Sergeant Act+ traits
/// C: Double Time: +1 more ATK on even turns (Rev Up grants +2 on even turns)
/// D: Cadence Call: +1 strike at Turn 5
pub static DRILL_SERGEANT_ACT_PLUS: ActPlusTraits = ActPlusTraits {
    act_c: &[
        // Double Time: +1 extra ATK every other turn (stacks with base +1 = +2 on even turns)
        BossTrait::new(TriggerType::EveryOtherTurn, EffectType::GainAtk, 1),
    ],
    act_d: &[
        // Cadence Call: +1 strike at Turn 5
        BossTrait::new(TriggerType::TurnN { turn: 5 }, EffectType::GainStrikes, 1),
    ],
};

/// Crystal Mimic Act+ traits
/// C: Mirror Sheen: +1 Reflection stack (3 instead of 2)
/// D: Shattered Reflection: When reflection depleted, deal 2 non-weapon damage (per stack consumed)
pub static CRYSTAL_MIMIC_ACT_PLUS: ActPlusTraits = ActPlusTraits {
    act_c: &[
        // Mirror Sheen: +1 Reflection at Battle Start (stacks with base 2 = 3)
        BossTrait::new(TriggerType::BattleStart, EffectType::ApplyReflection, 1)
            .with_special(SpecialMechanic::Reflection { stacks: 1 }),
    ],
    act_d: &[
        // Shattered Reflection: Deal 2 non-weapon damage when reflection is consumed
        BossTrait::new(
            TriggerType::TurnStart,
            EffectType::DealNonWeaponDamage,
            2,
        )
        .with_special(SpecialMechanic::Conditional {
            condition: Condition::ReflectionDepleted,
        }),
    ],
};

/// Rust Regent Act+ traits
/// C: Royal Decay: +1 more Rust on hit (2 instead of 1)
/// D: Tax Collector: Execution Tax deals +1 more damage; if you have Rust, +1 more
pub static RUST_REGENT_ACT_PLUS: ActPlusTraits = ActPlusTraits {
    act_c: &[
        // Royal Decay: +1 extra Rust on hit (stacks with base 1 = 2)
        BossTrait::new(TriggerType::OnHit, EffectType::ApplyRust, 1),
    ],
    act_d: &[
        // Tax Collector: +1 more Execution Tax damage (stacks with base 1 = 2)
        BossTrait::new(TriggerType::TurnStart, EffectType::DealDamage, 1).with_special(
            SpecialMechanic::Conditional {
                condition: Condition::PlayerExposed,
            },
        ),
    ],
};

/// Powder Keg Baron Act+ traits
/// C: Unstable Reserves: +2 more countdown damage (10 instead of 8)
/// D: Chain Reaction: On detonation, apply 2 Chill
pub static POWDER_KEG_BARON_ACT_PLUS: ActPlusTraits = ActPlusTraits {
    act_c: &[
        // Unstable Reserves: +2 non-weapon countdown damage (stacks with base 8 = 10)
        BossTrait::new(
            TriggerType::Countdown { turns: 3 },
            EffectType::DealNonWeaponDamage,
            2,
        )
        .with_special(SpecialMechanic::Countdown { turns: 3 }),
        // Also +2 self-damage
        BossTrait::new(
            TriggerType::Countdown { turns: 3 },
            EffectType::DealSelfNonWeaponDamage,
            2,
        )
        .with_special(SpecialMechanic::Countdown { turns: 3 }),
    ],
    act_d: &[
        // Chain Reaction: On detonation, apply 2 Chill
        BossTrait::new(
            TriggerType::Countdown { turns: 3 },
            EffectType::ApplyChill,
            2,
        )
        .with_special(SpecialMechanic::Countdown { turns: 3 }),
    ],
};

/// Greedkeeper Act+ traits
/// C: Vault Lock: Steal +4 more Gold (20 instead of 16), cap +1 (5 instead of 4)
/// D: Compound Interest: Every 3rd turn, +1 ATK
pub static GREEDKEEPER_ACT_PLUS: ActPlusTraits = ActPlusTraits {
    act_c: &[
        // Vault Lock: Steal 4 more Gold at Battle Start (stacks with base 16 = 20)
        BossTrait::new(TriggerType::BattleStart, EffectType::StealGold, 4),
        // Vault Lock: +1 more Armor from gold (cap increases by 1)
        BossTrait::new(TriggerType::BattleStart, EffectType::GainArmor, 1),
    ],
    act_d: &[
        // Compound Interest: Every 3rd turn, gain +1 ATK
        BossTrait::new(
            TriggerType::Countdown { turns: 3 },
            EffectType::GainAtk,
            1,
        ),
    ],
};

// --- W3 Finals ---

/// Eldritch Mole Act+ traits
/// C: Ancient Resilience: Phase thresholds shift to 80%/55%/30%
/// D: Primordial Wrath: Phase 3 Bleed increases to 3; at 10% HP +3 ATK
pub static ELDRITCH_MOLE_ACT_PLUS: ActPlusTraits = ActPlusTraits {
    act_c: &[
        // Ancient Resilience: Extra armor phase at 80% (instead of 75%)
        // Implemented as additional Phase(80) GainArmor to fire earlier
        BossTrait::new(TriggerType::TurnStart, EffectType::GainArmor, 6)
            .with_special(SpecialMechanic::Phase { threshold: 80 }),
    ],
    act_d: &[
        // Primordial Wrath: +1 more Bleed at Phase 3 (stacks with base 2 = 3)
        BossTrait::new(TriggerType::TurnStart, EffectType::ApplyBleed, 1)
            .with_special(SpecialMechanic::Phase { threshold: 25 }),
        // At 10% HP, gain +3 ATK
        BossTrait::new(TriggerType::TurnStart, EffectType::GainAtk, 3)
            .with_special(SpecialMechanic::Phase { threshold: 10 }),
    ],
};

/// Gilded Devourer Act+ traits
/// C: Insatiable: Gold→Armor improves to +1 per 2 Gold (cap 16)
/// D: Feast Frenzy: Wounded: deal 4 non-weapon damage AND apply 3 Bleed
pub static GILDED_DEVOURER_ACT_PLUS: ActPlusTraits = ActPlusTraits {
    act_c: &[
        // Insatiable: Additional armor from gold conversion (+4 cap increase)
        BossTrait::new(TriggerType::BattleStart, EffectType::GainArmor, 4).with_special(
            SpecialMechanic::GoldInteraction {
                steal_amount: 0,
                armor_ratio: 2,
            },
        ),
    ],
    act_d: &[
        // Feast Frenzy: Wounded: deal 4 non-weapon damage
        BossTrait::new(TriggerType::Wounded, EffectType::DealNonWeaponDamage, 4),
        // Feast Frenzy: Wounded: apply 3 Bleed (replaces base 2 with +1 extra)
        BossTrait::new(TriggerType::Wounded, EffectType::ApplyBleed, 1),
    ],
};

/// Frostbound Leviathan Act+ traits
/// C: Blizzard Pulse: Whiteout +1 Chill (3 instead of 2); Glacial Bulk +1 ARM (4 instead of 3)
/// D: Permafrost Shell: While 8+ ARM, reduce player weapon damage by 1
pub static FROSTBOUND_LEVIATHAN_ACT_PLUS: ActPlusTraits = ActPlusTraits {
    act_c: &[
        // Blizzard Pulse: +1 more Chill at Battle Start (stacks with base 2 = 3)
        BossTrait::new(TriggerType::BattleStart, EffectType::ApplyChill, 1),
        // Blizzard Pulse: +1 more Armor every other turn (stacks with base 3 = 4)
        BossTrait::new(TriggerType::EveryOtherTurn, EffectType::GainArmor, 1),
    ],
    act_d: &[
        // Permafrost Shell: While 8+ ARM, player weapon damage reduced by 1
        // This is modeled as a ReduceWeaponDamageWhileArmored effect on the boss
        BossTrait::new(
            TriggerType::BattleStart,
            EffectType::ReduceWeaponDamageWhileArmored,
            1,
        ),
    ],
};

/// Rusted Chronomancer Act+ traits
/// C: Temporal Loop: Time Shear also fires on Turn 5
/// D: Decaying Timeline: +1 more Rust/turn (2 instead of 1)
pub static RUSTED_CHRONOMANCER_ACT_PLUS: ActPlusTraits = ActPlusTraits {
    act_c: &[
        // Temporal Loop: +1 strike on Turn 5 (mirrors Turn 1 Time Shear)
        BossTrait::new(TriggerType::TurnN { turn: 5 }, EffectType::GainStrikes, 1),
    ],
    act_d: &[
        // Decaying Timeline: +1 more Rust per turn (stacks with base 1 = 2)
        BossTrait::new(TriggerType::TurnStart, EffectType::ApplyRust, 1),
    ],
};

/// Look up Act+ traits for a boss archetype by its ID prefix.
/// Returns None if the boss has no Act+ traits defined.
pub fn get_act_plus_traits(boss_id: &[u8; 12]) -> Option<&'static ActPlusTraits> {
    // Match by boss ID prefix (first 9 bytes determine the archetype)
    // Biome B W1/W2 bosses share Act+ traits with Biome A counterparts
    match &boss_id[..9] {
        // W1 bosses (both biomes share traits)
        b"B-A-W1-01" | b"B-B-W1-01" => Some(&BROODMOTHER_ACT_PLUS),
        b"B-A-W1-02" | b"B-B-W1-02" => Some(&OBSIDIAN_GOLEM_ACT_PLUS),
        b"B-A-W1-03" | b"B-B-W1-03" => Some(&GAS_ANOMALY_ACT_PLUS),
        b"B-A-W1-04" | b"B-B-W1-04" => Some(&MAD_MINER_ACT_PLUS),
        b"B-A-W1-05" | b"B-B-W1-05" => Some(&SHARD_COLOSSUS_ACT_PLUS),
        // W2 bosses (both biomes share traits)
        b"B-A-W2-01" | b"B-B-W2-01" => Some(&DRILL_SERGEANT_ACT_PLUS),
        b"B-A-W2-02" | b"B-B-W2-02" => Some(&CRYSTAL_MIMIC_ACT_PLUS),
        b"B-A-W2-03" | b"B-B-W2-03" => Some(&RUST_REGENT_ACT_PLUS),
        b"B-A-W2-04" | b"B-B-W2-04" => Some(&POWDER_KEG_BARON_ACT_PLUS),
        b"B-A-W2-05" | b"B-B-W2-05" => Some(&GREEDKEEPER_ACT_PLUS),
        // W3 finals (unique per biome)
        b"B-A-W3-01" => Some(&ELDRITCH_MOLE_ACT_PLUS),
        b"B-A-W3-02" => Some(&GILDED_DEVOURER_ACT_PLUS),
        b"B-B-W3-01" => Some(&FROSTBOUND_LEVIATHAN_ACT_PLUS),
        b"B-B-W3-02" => Some(&RUSTED_CHRONOMANCER_ACT_PLUS),
        _ => None,
    }
}

/// Phase levels for Week 3 finals
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    One,
    Two,
    Three,
}

/// Check for phase transitions based on HP percentage
pub fn check_phase_transition(hp: i16, max_hp: u16, state: &mut PhaseState) -> Option<Phase> {
    if hp <= 0 || max_hp == 0 {
        return None;
    }

    let hp_percent = ((hp as u32) * 100) / (max_hp as u32);

    // Check in order from lowest threshold to highest
    if hp_percent <= 25 && !state.phase_3_triggered {
        state.phase_3_triggered = true;
        return Some(Phase::Three);
    }
    if hp_percent <= 50 && !state.phase_2_triggered {
        state.phase_2_triggered = true;
        return Some(Phase::Two);
    }
    if hp_percent <= 75 && !state.phase_1_triggered {
        state.phase_1_triggered = true;
        return Some(Phase::One);
    }

    None
}

// ============================================================================
// Reflection State for Crystal Mimic
// ============================================================================

/// Tracks reflection stacks for Crystal Mimic
#[derive(Clone, Copy, Debug)]
pub struct ReflectionState {
    pub stacks: u8,
    pub glass_heart_active: bool,
}

impl Default for ReflectionState {
    fn default() -> Self {
        Self {
            stacks: 2,
            glass_heart_active: false,
        }
    }
}

// ============================================================================
// Countdown State for Powder Keg Baron
// ============================================================================

/// Tracks countdown traits
#[derive(Clone, Copy, Debug)]
pub struct CountdownState {
    pub trait_index: u8,
    pub turns_remaining: u8,
}

// ============================================================================
// Boss Combat State
// ============================================================================

/// Extended state for boss-specific mechanics during combat
#[derive(Clone, Debug, Default)]
pub struct BossCombatState {
    pub phase_state: PhaseState,
    pub reflection_state: Option<ReflectionState>,
    pub countdown_states: Vec<CountdownState>,
}
