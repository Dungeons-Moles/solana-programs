//! Combat effect generation for the player-inventory program.
//!
//! Converts equipped items and itemsets to ItemEffect arrays for the combat system.

use crate::items::get_item;
use crate::itemsets::get_active_itemsets;
use crate::state::{
    stat_bonus, EffectType, ItemEffect, ItemInstance, PlayerInventory, Tier, ToolOilModification,
};

/// Convert an item's effects to ItemEffect array with tier scaling
pub fn generate_item_effects(item: &ItemInstance) -> Vec<ItemEffect> {
    let Some(definition) = get_item(&item.item_id) else {
        return Vec::new();
    };

    definition
        .effects
        .iter()
        .map(|effect_def| effect_def.to_item_effect(item.tier))
        .collect()
}

/// Generate effects for the equipped tool
pub fn generate_tool_effects(tool: &ItemInstance) -> Vec<ItemEffect> {
    let mut effects = generate_item_effects(tool);

    // Add Tool Oil bonuses
    if tool.has_oil(ToolOilModification::PlusAtk) {
        effects.push(stat_bonus(EffectType::GainAtk, 1));
    }
    if tool.has_oil(ToolOilModification::PlusSpd) {
        effects.push(stat_bonus(EffectType::GainSpd, 1));
    }
    if tool.has_oil(ToolOilModification::PlusDig) {
        effects.push(stat_bonus(EffectType::GainDig, 1));
    }

    effects
}

/// Generate effects for all equipped gear
pub fn generate_gear_effects(gear_slots: &[Option<ItemInstance>]) -> Vec<ItemEffect> {
    gear_slots
        .iter()
        .flatten()
        .flat_map(generate_item_effects)
        .collect()
}

/// Generate effects for all active itemsets
pub fn generate_itemset_effects(inventory: &PlayerInventory) -> Vec<ItemEffect> {
    let active_sets = get_active_itemsets(inventory);

    active_sets
        .into_iter()
        .flat_map(|set| {
            set.bonus_effect
                .iter()
                .map(|effect_def| effect_def.to_item_effect(Tier::I))
        })
        .collect()
}

/// Generate all combat effects from the player's inventory
///
/// This is the main entry point for combat integration.
/// Returns a Vec<ItemEffect> that can be passed directly to the combat system.
pub fn generate_combat_effects(inventory: &PlayerInventory) -> Vec<ItemEffect> {
    let mut effects = Vec::new();

    // 1. Add tool effects
    if let Some(ref tool) = inventory.tool {
        effects.extend(generate_tool_effects(tool));
    }

    // 2. Add gear effects
    effects.extend(generate_gear_effects(&inventory.gear));

    // 3. Add itemset bonuses
    effects.extend(generate_itemset_effects(inventory));

    effects
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::prelude::Pubkey;

    fn make_inventory() -> PlayerInventory {
        PlayerInventory {
            player: Pubkey::default(),
            tool: None,
            gear: [None; 8],
            gear_slot_capacity: 4,
            bump: 0,
        }
    }

    #[test]
    fn test_tool_effects_with_tier_scaling() {
        // T-FR-01 (Rime Pike): +2/3/4 ATK, OnHit 1/2/3 Chill
        let tool = ItemInstance::new(*b"T-FR-01\0", Tier::II);
        let effects = generate_tool_effects(&tool);

        // Should have 2 effects (ATK and Chill)
        assert_eq!(effects.len(), 2);

        // Check ATK bonus uses Tier II value (3)
        let atk_effect = effects
            .iter()
            .find(|e| e.effect_type == EffectType::GainAtk);
        assert!(atk_effect.is_some());
        assert_eq!(atk_effect.unwrap().value, 3);

        // Check Chill uses Tier II value (2)
        let chill_effect = effects
            .iter()
            .find(|e| e.effect_type == EffectType::ApplyChill);
        assert!(chill_effect.is_some());
        assert_eq!(chill_effect.unwrap().value, 2);
    }

    #[test]
    fn test_tool_oil_effects() {
        let mut tool = ItemInstance::new(*b"T-FR-01\0", Tier::I);
        tool.apply_oil(ToolOilModification::PlusAtk);
        tool.apply_oil(ToolOilModification::PlusSpd);

        let effects = generate_tool_effects(&tool);

        // Should have base effects + 2 oil effects
        let atk_effects: Vec<_> = effects
            .iter()
            .filter(|e| e.effect_type == EffectType::GainAtk)
            .collect();
        assert_eq!(atk_effects.len(), 2); // Base +2 ATK, Oil +1 ATK

        let spd_effects: Vec<_> = effects
            .iter()
            .filter(|e| e.effect_type == EffectType::GainSpd)
            .collect();
        assert_eq!(spd_effects.len(), 1); // Oil +1 SPD
    }

    #[test]
    fn test_gear_effects_accumulation() {
        let mut inventory = make_inventory();

        // G-ST-01 (Miner Helmet): +3/6/9 ARM
        inventory.gear[0] = Some(ItemInstance::new(*b"G-ST-01\0", Tier::I));
        // G-ST-02 (Work Vest): +2/4/6 ARM, +2/3/4 Heal
        inventory.gear[1] = Some(ItemInstance::new(*b"G-ST-02\0", Tier::I));

        let effects = generate_gear_effects(&inventory.gear);

        // Should have 3 effects total (1 from Helmet, 2 from Vest)
        assert_eq!(effects.len(), 3);

        // Check armor effects
        let armor_effects: Vec<_> = effects
            .iter()
            .filter(|e| e.effect_type == EffectType::GainArmor)
            .collect();
        assert_eq!(armor_effects.len(), 2);
    }

    #[test]
    fn test_itemset_bonus_inclusion() {
        let mut inventory = make_inventory();

        // Union Standard set: G-ST-01, G-ST-02, G-SC-01
        inventory.gear[0] = Some(ItemInstance::new(*b"G-ST-01\0", Tier::I));
        inventory.gear[1] = Some(ItemInstance::new(*b"G-ST-02\0", Tier::I));
        inventory.gear[2] = Some(ItemInstance::new(*b"G-SC-01\0", Tier::I));

        let effects = generate_itemset_effects(&inventory);

        // Union Standard gives +4 ARM, +1 DIG at battle start
        assert_eq!(effects.len(), 2);

        let armor_bonus = effects
            .iter()
            .find(|e| e.effect_type == EffectType::GainArmor);
        assert!(armor_bonus.is_some());
        assert_eq!(armor_bonus.unwrap().value, 4);

        let dig_bonus = effects
            .iter()
            .find(|e| e.effect_type == EffectType::GainDig);
        assert!(dig_bonus.is_some());
        assert_eq!(dig_bonus.unwrap().value, 1);
    }

    #[test]
    fn test_full_combat_effects() {
        let mut inventory = make_inventory();

        // Equip tool
        inventory.tool = Some(ItemInstance::new(*b"T-FR-01\0", Tier::I));

        // Equip gear for Union Standard
        inventory.gear[0] = Some(ItemInstance::new(*b"G-ST-01\0", Tier::I));
        inventory.gear[1] = Some(ItemInstance::new(*b"G-ST-02\0", Tier::I));
        inventory.gear[2] = Some(ItemInstance::new(*b"G-SC-01\0", Tier::I));

        let effects = generate_combat_effects(&inventory);

        // Should have:
        // - 2 from tool (ATK, Chill)
        // - 1 from G-ST-01 (ARM)
        // - 2 from G-ST-02 (ARM, Heal)
        // - 1 from G-SC-01 (DIG)
        // - 2 from Union Standard (ARM, DIG)
        // Total: 8 effects
        assert_eq!(effects.len(), 8);
    }

    #[test]
    fn test_empty_inventory() {
        let inventory = make_inventory();
        let effects = generate_combat_effects(&inventory);
        assert!(effects.is_empty());
    }
}
