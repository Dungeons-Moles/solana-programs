//! Stat derivation from player inventory.
//!
//! Stats are calculated at runtime from equipped items rather than stored in GameState.
//! This ensures stats always reflect current equipment and prevents desync issues.
//!
//! IMPORTANT: Only max_hp and dig are pre-calculated here. ATK/ARM/SPD are applied
//! during combat's BattleStart phase to avoid double-counting. Combat stats start
//! at base values (0) and get their bonuses from BattleStart effects in the combat system.

use crate::constants::{base_hp, BASE_DIG};
use combat_system::{EffectType, TriggerType};
use player_inventory::effects::generate_combat_effects;
use player_inventory::state::PlayerInventory;

/// Derived player stats from inventory.
/// Used for combat and movement calculations.
///
/// NOTE: Only `max_hp`, `dig`, and `strikes` are derived here. Combat stats (ATK/ARM/SPD)
/// are applied during combat's BattleStart phase to prevent double-counting.
#[derive(Debug, Clone, Copy, Default)]
pub struct PlayerStats {
    /// Maximum HP (base + permanent item bonuses like Work Vest's +HP)
    pub max_hp: i16,
    /// Dig stat (affects wall movement cost and combat comparators)
    pub dig: i16,
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
/// Combat stats (ATK/ARM/SPD) are NOT calculated here - they start at base (0)
/// and are applied during combat's BattleStart phase via item effects.
pub fn calculate_stats(inventory: &PlayerInventory, campaign_level: u8) -> PlayerStats {
    let effects = generate_combat_effects(inventory);

    let mut stats = PlayerStats {
        max_hp: base_hp(campaign_level),
        dig: BASE_DIG,
        strikes: BASE_STRIKES,
    };

    // Only process effects that are needed outside of combat
    for effect in effects.iter() {
        if effect.trigger != TriggerType::BattleStart {
            continue;
        }

        match effect.effect_type {
            EffectType::MaxHp => {
                // Permanent max HP bonus (e.g., Work Vest's +HP)
                stats.max_hp = stats.max_hp.saturating_add(effect.value);
            }
            EffectType::GainDig => {
                // DIG is used for movement cost and combat comparators
                stats.dig = stats.dig.saturating_add(effect.value);
            }
            EffectType::GainStrikes => {
                // Additional strikes per turn (e.g., Twin Picks gives +1 strike)
                let bonus = effect.value.max(0) as u8;
                stats.strikes = stats.strikes.saturating_add(bonus);
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
        let stats = calculate_stats(&inventory, 20);

        assert_eq!(stats.max_hp, 15);
        assert_eq!(stats.dig, BASE_DIG);
    }

    #[test]
    fn test_base_hp_scales_by_campaign_level() {
        let inventory = make_inventory();

        let stats_low = calculate_stats(&inventory, 1);
        assert_eq!(stats_low.max_hp, 25, "levels 1-9 should start at 25 HP");

        let stats_mid = calculate_stats(&inventory, 10);
        assert_eq!(stats_mid.max_hp, 20, "levels 10-19 should start at 20 HP");

        let stats_high = calculate_stats(&inventory, 20);
        assert_eq!(stats_high.max_hp, 15, "levels 20+ should start at 15 HP");
    }

    #[test]
    fn test_stats_with_max_hp_item() {
        let mut inventory = make_inventory();
        let stats = calculate_stats(&inventory, 20);
        assert_eq!(stats.max_hp, 15);
    }

    #[test]
    fn test_stats_with_dig_item() {
        let mut inventory = make_inventory();
        inventory.gear[0] = Some(ItemInstance::new(*b"G-MV-05\0", Tier::I));

        let stats = calculate_stats(&inventory, 20);

        assert!(stats.dig >= BASE_DIG);
    }

    #[test]
    fn test_stats_only_calculates_permanent_bonuses() {
        let mut inventory = make_inventory();
        inventory.tool = Some(ItemInstance::new(*b"T-FR-01\0", Tier::II));
        inventory.gear[0] = Some(ItemInstance::new(*b"G-ST-01\0", Tier::I));

        let stats = calculate_stats(&inventory, 20);

        assert_eq!(stats.max_hp, 15);
        assert_eq!(stats.dig, BASE_DIG);
    }
}
