// Traits module - Enemy trait definitions
// Implementation in US3 (T040-T052)

use combat_system::state::{EffectType, ItemEffect, TriggerType};

use crate::archetypes::ids;

/// Tunnel Rat: On Hit (once/turn): steal 1 Gold
pub static TUNNEL_RAT_TRAITS: [ItemEffect; 1] = [ItemEffect {
    trigger: TriggerType::OnHit,
    once_per_turn: true,
    effect_type: EffectType::StealGold,
    value: 1,
}];

/// Cave Bat: Every other turn: restore 1 HP
pub static CAVE_BAT_TRAITS: [ItemEffect; 1] = [ItemEffect {
    trigger: TriggerType::EveryOtherTurn,
    once_per_turn: false,
    effect_type: EffectType::Heal,
    value: 1,
}];

/// Spore Slime: Battle Start: apply 2 Chill to player
pub static SPORE_SLIME_TRAITS: [ItemEffect; 1] = [ItemEffect {
    trigger: TriggerType::BattleStart,
    once_per_turn: false,
    effect_type: EffectType::ApplyChill,
    value: 2,
}];

/// Rust Mite Swarm: On Hit (once/turn): apply 1 Rust
pub static RUST_MITE_TRAITS: [ItemEffect; 1] = [ItemEffect {
    trigger: TriggerType::OnHit,
    once_per_turn: true,
    effect_type: EffectType::ApplyRust,
    value: 1,
}];

/// Collapsed Miner: Wounded: gain +3 ATK (this battle)
pub static COLLAPSED_MINER_TRAITS: [ItemEffect; 1] = [ItemEffect {
    trigger: TriggerType::Wounded,
    once_per_turn: false,
    effect_type: EffectType::GainAtk,
    value: 3,
}];

/// Shard Beetle: Battle Start: gain 6 Shrapnel
pub static SHARD_BEETLE_TRAITS: [ItemEffect; 1] = [ItemEffect {
    trigger: TriggerType::BattleStart,
    once_per_turn: false,
    effect_type: EffectType::ApplyShrapnel,
    value: 6,
}];

/// Tunnel Warden: First strike each turn: remove 3 Armor from player before damage
pub static TUNNEL_WARDEN_TRAITS: [ItemEffect; 1] = [ItemEffect {
    trigger: TriggerType::TurnStart,
    once_per_turn: true,
    effect_type: EffectType::RemoveArmor,
    value: 3,
}];

/// Burrow Ambusher: Battle Start: deal 3 damage ignoring Armor
pub static BURROW_AMBUSHER_TRAITS: [ItemEffect; 1] = [ItemEffect {
    trigger: TriggerType::BattleStart,
    once_per_turn: false,
    effect_type: EffectType::DealNonWeaponDamage,
    value: 3,
}];

/// Frost Wisp: If it acts first on Turn 1: apply 2 Chill
/// Note: FirstTurn trigger + SPD check handled in combat engine
pub static FROST_WISP_TRAITS: [ItemEffect; 1] = [ItemEffect {
    trigger: TriggerType::FirstTurn,
    once_per_turn: false,
    effect_type: EffectType::ApplyChill,
    value: 2,
}];

/// Powder Tick: Countdown(2): deal 6 damage to player (non-weapon)
/// Fires every 2 turns (turn 2, 4, 6, etc.)
pub static POWDER_TICK_TRAITS: [ItemEffect; 1] = [ItemEffect {
    trigger: TriggerType::Countdown { turns: 2 },
    once_per_turn: false,
    effect_type: EffectType::DealNonWeaponDamage,
    value: 6,
}];

/// Coin Slug: Battle Start: gain Armor equal to floor(player Gold/10) (cap 3)
/// Note: Gold-based armor calculation handled in combat initialization
pub static COIN_SLUG_TRAITS: [ItemEffect; 1] = [ItemEffect {
    trigger: TriggerType::BattleStart,
    once_per_turn: false,
    effect_type: EffectType::GainArmor,
    value: 0, // Calculated dynamically based on player gold
}];

/// Blood Mosquito: On Hit (once/turn): apply 1 Bleed
pub static BLOOD_MOSQUITO_TRAITS: [ItemEffect; 1] = [ItemEffect {
    trigger: TriggerType::OnHit,
    once_per_turn: true,
    effect_type: EffectType::ApplyBleed,
    value: 1,
}];


/// Crystal Mimic: Battle Start: gain 1 Reflection
pub static CRYSTAL_MIMIC_TRAITS: [ItemEffect; 1] = [ItemEffect {
    trigger: TriggerType::BattleStart,
    once_per_turn: false,
    effect_type: EffectType::ApplyReflection,
    value: 1,
}];

/// Get the trait effects for an enemy archetype
pub fn get_enemy_traits(archetype_id: u8) -> &'static [ItemEffect] {
    match archetype_id {
        ids::TUNNEL_RAT => &TUNNEL_RAT_TRAITS,
        ids::CAVE_BAT => &CAVE_BAT_TRAITS,
        ids::SPORE_SLIME => &SPORE_SLIME_TRAITS,
        ids::RUST_MITE_SWARM => &RUST_MITE_TRAITS,
        ids::COLLAPSED_MINER => &COLLAPSED_MINER_TRAITS,
        ids::SHARD_BEETLE => &SHARD_BEETLE_TRAITS,
        ids::TUNNEL_WARDEN => &TUNNEL_WARDEN_TRAITS,
        ids::BURROW_AMBUSHER => &BURROW_AMBUSHER_TRAITS,
        ids::FROST_WISP => &FROST_WISP_TRAITS,
        ids::POWDER_TICK => &POWDER_TICK_TRAITS,
        ids::COIN_SLUG => &COIN_SLUG_TRAITS,
        ids::BLOOD_MOSQUITO => &BLOOD_MOSQUITO_TRAITS,
        ids::CRYSTAL_MIMIC => &CRYSTAL_MIMIC_TRAITS,
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shard_beetle_trait() {
        let traits = get_enemy_traits(ids::SHARD_BEETLE);
        assert_eq!(traits.len(), 1);
        assert!(matches!(traits[0].trigger, TriggerType::BattleStart));
        assert!(matches!(traits[0].effect_type, EffectType::ApplyShrapnel));
        assert_eq!(traits[0].value, 6);
    }

    #[test]
    fn test_blood_mosquito_trait() {
        let traits = get_enemy_traits(ids::BLOOD_MOSQUITO);
        assert_eq!(traits.len(), 1);
        assert!(matches!(traits[0].trigger, TriggerType::OnHit));
        assert!(traits[0].once_per_turn);
        assert!(matches!(traits[0].effect_type, EffectType::ApplyBleed));
        assert_eq!(traits[0].value, 1);
    }

    #[test]
    fn test_spore_slime_trait() {
        let traits = get_enemy_traits(ids::SPORE_SLIME);
        assert_eq!(traits.len(), 1);
        assert!(matches!(traits[0].trigger, TriggerType::BattleStart));
        assert!(matches!(traits[0].effect_type, EffectType::ApplyChill));
        assert_eq!(traits[0].value, 2);
    }

    #[test]
    fn test_all_archetypes_have_traits() {
        for id in 0..13u8 {
            let traits = get_enemy_traits(id);
            assert!(!traits.is_empty(), "Archetype {} has no traits", id);
        }
    }

    #[test]
    fn test_invalid_archetype_returns_empty() {
        let traits = get_enemy_traits(13);
        assert!(traits.is_empty());
        let traits = get_enemy_traits(255);
        assert!(traits.is_empty());
    }
}
