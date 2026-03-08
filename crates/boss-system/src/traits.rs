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

/// Drill Sergeant: +1 ATK +1 SPD/turn, +2 ARM/2 turns
pub static DRILL_SERGEANT_TRAITS: &[BossTrait] = &[
    // Rev Up: +1 ATK at Turn Start
    BossTrait::new(TriggerType::TurnStart, EffectType::GainAtk, 1),
    // Rev Up: +1 SPD at Turn Start
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

/// Greedkeeper: Steal 10 Gold, ARM = stolen/5 (cap 6)
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

/// The Eldritch Mole: Three phases (75%: +10 ARM, 50%: 2 strikes, 25%: 2 Bleed/turn)
/// Deep Dig: Player DIG > boss DIG reduces Phase 1 armor by 10
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

/// The Gilded Devourer: Gold→ARM (+1/5 cap 10), Wounded: 3 Bleed
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

/// The Frostbound Leviathan: 3 Chill start, +4 ARM/2 turns, Exposed: clear Chill +2 SPD
pub static FROSTBOUND_LEVIATHAN_TRAITS: &[BossTrait] = &[
    // Whiteout: 2 Chill at Battle Start
    BossTrait::new(TriggerType::BattleStart, EffectType::ApplyChill, 2),
    // Glacial Bulk: +3 ARM every 2 turns
    BossTrait::new(TriggerType::EveryOtherTurn, EffectType::GainArmor, 3),
    // Crack Ice: First time Exposed, clear Chill and +2 SPD
    BossTrait::new(TriggerType::FirstTimeExposed, EffectType::GainSpd, 2),
];

/// The Rusted Chronomancer: 2 strikes T1, 1 Rust/turn, Wounded: 4 Bleed
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
