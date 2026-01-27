//! Stat derivation from player inventory.
//!
//! Stats are calculated at runtime from equipped items rather than stored in GameState.
//! This ensures stats always reflect current equipment and prevents desync issues.

use crate::constants::{BASE_ARM, BASE_ATK, BASE_DIG, BASE_HP, BASE_SPD};
use combat_system::{EffectType, TriggerType};
use player_inventory::effects::generate_combat_effects;
use player_inventory::state::PlayerInventory;

/// Derived player stats from inventory.
/// Used for combat and movement calculations.
#[derive(Debug, Clone, Copy, Default)]
pub struct PlayerStats {
    /// Maximum HP (base + item bonuses)
    pub max_hp: i16,
    /// Attack stat
    pub atk: i16,
    /// Armor stat
    pub arm: i16,
    /// Speed stat
    pub spd: i16,
    /// Dig stat (affects wall movement cost)
    pub dig: i16,
}

/// Calculate player stats by summing base values with item effect bonuses.
///
/// This function iterates over all equipped items (tool + gear + itemsets)
/// and sums BattleStart effects that modify stats.
pub fn calculate_stats(inventory: &PlayerInventory) -> PlayerStats {
    let effects = generate_combat_effects(inventory);

    let mut stats = PlayerStats {
        max_hp: BASE_HP,
        atk: BASE_ATK,
        arm: BASE_ARM,
        spd: BASE_SPD,
        dig: BASE_DIG,
    };

    // Sum up stat bonuses from BattleStart effects
    for effect in effects.iter() {
        if effect.trigger != TriggerType::BattleStart {
            continue;
        }

        match effect.effect_type {
            EffectType::Heal => {
                // Heal at BattleStart represents bonus max HP (e.g., Health Ring)
                stats.max_hp = stats.max_hp.saturating_add(effect.value);
            }
            EffectType::GainAtk => {
                stats.atk = stats.atk.saturating_add(effect.value);
            }
            EffectType::GainArmor => {
                stats.arm = stats.arm.saturating_add(effect.value);
            }
            EffectType::GainSpd => {
                stats.spd = stats.spd.saturating_add(effect.value);
            }
            EffectType::GainDig => {
                stats.dig = stats.dig.saturating_add(effect.value);
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
            gear: [None; 8],
            gear_slot_capacity: 4,
            bump: 0,
        }
    }

    #[test]
    fn test_base_stats_empty_inventory() {
        let inventory = make_inventory();
        let stats = calculate_stats(&inventory);

        assert_eq!(stats.max_hp, BASE_HP);
        assert_eq!(stats.atk, BASE_ATK);
        assert_eq!(stats.arm, BASE_ARM);
        assert_eq!(stats.spd, BASE_SPD);
        assert_eq!(stats.dig, BASE_DIG);
    }

    #[test]
    fn test_stats_with_tool() {
        let mut inventory = make_inventory();
        // T-FR-01 (Rime Pike): +2/3/4 ATK at BattleStart
        inventory.tool = Some(ItemInstance::new(*b"T-FR-01\0", Tier::I));

        let stats = calculate_stats(&inventory);

        // Base ATK (0) + Rime Pike Tier I (+2) = 2
        assert_eq!(stats.atk, 2);
    }

    #[test]
    fn test_stats_with_gear() {
        let mut inventory = make_inventory();
        // G-ST-01 (Miner Helmet): +3/6/9 ARM at BattleStart
        inventory.gear[0] = Some(ItemInstance::new(*b"G-ST-01\0", Tier::I));

        let stats = calculate_stats(&inventory);

        // Base ARM (0) + Miner Helmet Tier I (+3) = 3
        assert_eq!(stats.arm, 3);
    }

    #[test]
    fn test_stats_accumulation() {
        let mut inventory = make_inventory();
        // Tool with ATK
        inventory.tool = Some(ItemInstance::new(*b"T-FR-01\0", Tier::II)); // +3 ATK
                                                                           // Gear with ARM
        inventory.gear[0] = Some(ItemInstance::new(*b"G-ST-01\0", Tier::I)); // +3 ARM
        inventory.gear[1] = Some(ItemInstance::new(*b"G-ST-02\0", Tier::I)); // +2 ARM

        let stats = calculate_stats(&inventory);

        assert_eq!(stats.atk, 3); // Tool bonus
        assert_eq!(stats.arm, 5); // 3 + 2 from gear
    }
}
