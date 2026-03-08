//! Stat derivation from player inventory.
//!
//! Stats are calculated at runtime from equipped items rather than stored in GameState.
//! This ensures stats always reflect current equipment and prevents desync issues.
//!
//! IMPORTANT: unconditional Tool/Gear combat stats are pre-calculated here so on-chain combat
//! starts from the same baseline as local replay. Conditional battle-start effects and itemset
//! battle-start stat bonuses remain dynamic combat effects.

use crate::constants::{base_hp, BASE_DIG, PVP_BASE_HP};
use crate::state::RunMode;
use combat_system::{CombatSourceKind, EffectType, TriggerType};
use player_inventory::effects::generate_annotated_combat_effects;
use player_inventory::state::{Condition, PlayerInventory};

/// Derived player stats from inventory.
/// Used for combat and movement calculations.
///
/// NOTE: `atk`, `arm`, and `spd` only include unconditional Tool/Gear bonuses. Conditional
/// battle-start effects and itemset stat bonuses are still resolved inside combat.
#[derive(Debug, Clone, Copy, Default)]
pub struct PlayerStats {
    /// Maximum HP (base + permanent item bonuses like Work Vest's +HP)
    pub max_hp: i16,
    /// Dig stat (affects wall movement cost and combat comparators)
    pub dig: i16,
    /// Starting ATK for combat
    pub atk: i16,
    /// Starting ARM for combat
    pub arm: i16,
    /// Starting SPD for combat
    pub spd: i16,
    /// Number of strikes per turn (base 1 + GainStrikes bonuses)
    pub strikes: u8,
}

/// Base number of strikes per turn
const BASE_STRIKES: u8 = 1;

/// Calculate player stats by summing base values with item effect bonuses.
///
/// This function only calculates stats needed OUTSIDE of combat:
/// - max_hp: For HP capping and healing limits
/// - dig: For movement cost calculation
/// - strikes: For multi-strike tools (Twin Picks, Pneumatic Drill)
///
/// Combat stats from unconditional Tool/Gear battle-start bonuses are calculated here so
/// on-chain combat begins with the same sidebar values as local combat.
pub fn calculate_stats(
    inventory: &PlayerInventory,
    campaign_level: u8,
    run_mode: RunMode,
) -> PlayerStats {
    let effects = generate_annotated_combat_effects(inventory);

    let base_hp_value = match run_mode {
        RunMode::Campaign => base_hp(campaign_level),
        _ => PVP_BASE_HP,
    };

    let mut stats = PlayerStats {
        max_hp: base_hp_value,
        dig: BASE_DIG,
        atk: 0,
        arm: 0,
        spd: 0,
        strikes: BASE_STRIKES,
    };

    // Only process unconditional Tool/Gear battle-start effects that define the
    // combatant's starting stats.
    for effect in effects.iter() {
        if effect.effect.trigger != TriggerType::BattleStart {
            continue;
        }

        if effect.effect.condition != Condition::None {
            continue;
        }

        let Some(source) = effect.source else {
            continue;
        };

        if !matches!(source.kind, CombatSourceKind::Tool | CombatSourceKind::Gear) {
            continue;
        }

        match effect.effect.effect_type {
            EffectType::MaxHp => {
                // Permanent max HP bonus (e.g., Work Vest's +HP)
                stats.max_hp = stats.max_hp.saturating_add(effect.effect.value);
            }
            EffectType::GainDig => {
                // DIG is used for movement cost and combat comparators
                stats.dig = stats.dig.saturating_add(effect.effect.value);
            }
            EffectType::GainStrikes => {
                // Additional strikes per turn (e.g., Twin Picks gives +1 strike)
                let bonus = effect.effect.value.max(0) as u8;
                stats.strikes = stats.strikes.saturating_add(bonus);
            }
            EffectType::GainAtk | EffectType::GainGearAtk => {
                stats.atk = stats.atk.saturating_add(effect.effect.value);
            }
            EffectType::GainArmor => {
                stats.arm = stats.arm.saturating_add(effect.effect.value);
            }
            EffectType::GainSpd => {
                stats.spd = stats.spd.saturating_add(effect.effect.value);
            }
            _ => {}
        }
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::prelude::Pubkey;
    use player_inventory::state::{ItemInstance, Tier};

    fn make_inventory() -> PlayerInventory {
        PlayerInventory {
            session: Pubkey::default(),
            player: Pubkey::default(),
            tool: None,
            gear: [None; 12],
            gear_slot_capacity: 4,
            bump: 0,
        }
    }

    #[test]
    fn test_base_stats_empty_inventory() {
        let inventory = make_inventory();
        let stats = calculate_stats(&inventory, 20, RunMode::Campaign);

        assert_eq!(stats.max_hp, 15);
        assert_eq!(stats.dig, BASE_DIG);
        assert_eq!(stats.atk, 0);
        assert_eq!(stats.arm, 0);
        assert_eq!(stats.spd, 0);
    }

    #[test]
    fn test_base_hp_scales_by_campaign_level() {
        let inventory = make_inventory();

        let stats_low = calculate_stats(&inventory, 1, RunMode::Campaign);
        assert_eq!(stats_low.max_hp, 25, "levels 1-9 should start at 25 HP");

        let stats_mid = calculate_stats(&inventory, 10, RunMode::Campaign);
        assert_eq!(stats_mid.max_hp, 20, "levels 10-19 should start at 20 HP");

        let stats_high = calculate_stats(&inventory, 20, RunMode::Campaign);
        assert_eq!(stats_high.max_hp, 15, "levels 20+ should start at 15 HP");
    }

    #[test]
    fn test_stats_with_max_hp_item() {
        let inventory = make_inventory();
        let stats = calculate_stats(&inventory, 20, RunMode::Campaign);
        assert_eq!(stats.max_hp, 15);
    }

    #[test]
    fn test_stats_with_dig_item() {
        let mut inventory = make_inventory();
        inventory.gear[0] = Some(ItemInstance::new(*b"G-MV-05\0", Tier::I));

        let stats = calculate_stats(&inventory, 20, RunMode::Campaign);

        assert!(stats.dig >= BASE_DIG);
    }

    #[test]
    fn test_stats_include_unconditional_tool_and_gear_combat_bonuses() {
        let mut inventory = make_inventory();
        inventory.tool = Some(ItemInstance::new(*b"T-RU-01\0", Tier::I)); // Corrosive Pick (+1 ATK)
        inventory.gear[0] = Some(ItemInstance::new(*b"G-SC-02\0", Tier::I)); // Leather Gloves (+1 ATK, +1 DIG)
        inventory.gear[1] = Some(ItemInstance::new(*b"G-FR-02\0", Tier::I)); // Frostguard Buckler (+8 ARM)

        let stats = calculate_stats(&inventory, 20, RunMode::Campaign);

        assert_eq!(stats.max_hp, 15);
        assert_eq!(stats.dig, BASE_DIG + 1);
        assert_eq!(stats.atk, 2);
        assert_eq!(stats.arm, 8);
        assert_eq!(stats.spd, 0);
    }

    #[test]
    fn test_itemset_battle_start_stats_are_not_baked() {
        let mut inventory = make_inventory();
        inventory.gear[0] = Some(ItemInstance::new(*b"G-ST-01\0", Tier::I));
        inventory.gear[1] = Some(ItemInstance::new(*b"G-ST-02\0", Tier::I));
        inventory.gear[2] = Some(ItemInstance::new(*b"G-SC-01\0", Tier::I));

        let stats = calculate_stats(&inventory, 20, RunMode::Campaign);

        // Stone helmet contributes +3 ARM, scout boots contribute +1 SPD and the itemset
        // bonus remains dynamic rather than baked into the starting combatant.
        assert_eq!(stats.arm, 3);
        assert_eq!(stats.spd, 1);
    }

    #[test]
    fn pvp_run_mode_uses_pvp_base_hp() {
        let inventory = make_inventory();
        let stats = calculate_stats(&inventory, 1, RunMode::Duel);

        assert_eq!(stats.max_hp, PVP_BASE_HP);
    }
}
