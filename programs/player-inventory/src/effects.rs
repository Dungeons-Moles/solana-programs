//! Combat effect generation for the player-inventory program.
//!
//! Converts equipped items and itemsets to ItemEffect arrays for the combat system.

use crate::items::get_item;
use crate::itemsets::get_active_itemsets;
use crate::nft_items::get_nft_item;
use crate::state::{
    stat_bonus, EffectType, ItemEffect, ItemInstance, PlayerInventory, Tier, ToolOilModification,
};
use combat_system::state::{AnnotatedItemEffect, CombatSourceKind, CombatSourceRef};

const GEMFINDER_STAFF_ID: [u8; 8] = *b"T-GR-02\0";
const EMERALD_SHARD_ID: [u8; 8] = *b"G-GR-05\0";
const RUBY_SHARD_ID: [u8; 8] = *b"G-GR-06\0";
const SAPPHIRE_SHARD_ID: [u8; 8] = *b"G-GR-07\0";
const CITRINE_SHARD_ID: [u8; 8] = *b"G-GR-08\0";

fn item_source(kind: CombatSourceKind, item_id: &[u8; 8]) -> CombatSourceRef {
    let mut id = [0u8; 16];
    id[..8].copy_from_slice(item_id);
    CombatSourceRef { kind, id }
}

fn itemset_source(itemset_id: &str) -> CombatSourceRef {
    let mut id = [0u8; 16];
    let bytes = itemset_id.as_bytes();
    let len = bytes.len().min(16);
    id[..len].copy_from_slice(&bytes[..len]);
    CombatSourceRef {
        kind: CombatSourceKind::Itemset,
        id,
    }
}

fn annotate_effects(effects: Vec<ItemEffect>, source: CombatSourceRef) -> Vec<AnnotatedItemEffect> {
    effects
        .into_iter()
        .map(|effect| AnnotatedItemEffect {
            effect,
            source: Some(source),
        })
        .collect()
}

/// Convert an item's effects to ItemEffect array with tier scaling
pub fn generate_item_effects(item: &ItemInstance) -> Vec<ItemEffect> {
    // Try base item registry first
    if let Some(definition) = get_item(&item.item_id) {
        return definition
            .effects
            .iter()
            .map(|effect_def| effect_def.to_item_effect(item.tier))
            .collect();
    }

    // Try NFT item registry
    if let Some(definition) = get_nft_item(&item.item_id) {
        return definition
            .effects
            .iter()
            .map(|effect_def| effect_def.to_item_effect(item.tier))
            .collect();
    }

    Vec::new()
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
    if tool.has_oil(ToolOilModification::PlusArm) {
        effects.push(stat_bonus(EffectType::GainArmor, 1));
    }

    effects
}

/// Generate effects for all equipped gear
fn apply_gemfinder_shard_bonus(item: &ItemInstance, effects: &mut [ItemEffect]) {
    for effect in effects.iter_mut() {
        match item.item_id {
            EMERALD_SHARD_ID
                if effect.trigger == crate::state::TriggerType::EveryOtherTurnFirstHit
                    && effect.effect_type == EffectType::Heal =>
            {
                effect.value = effect.value.saturating_add(1);
            }
            RUBY_SHARD_ID
                if effect.trigger == crate::state::TriggerType::EveryOtherTurnFirstHit
                    && effect.effect_type == EffectType::DealNonWeaponDamage =>
            {
                effect.value = effect.value.saturating_add(1);
            }
            SAPPHIRE_SHARD_ID
                if effect.trigger == crate::state::TriggerType::EveryOtherTurnFirstHit
                    && effect.effect_type == EffectType::GainArmor =>
            {
                effect.value = effect.value.saturating_add(1);
            }
            CITRINE_SHARD_ID
                if effect.trigger == crate::state::TriggerType::EveryOtherTurnFirstHit
                    && effect.effect_type == EffectType::GainGold =>
            {
                effect.value = effect.value.saturating_add(1);
            }
            _ => {}
        }
    }
}

pub fn generate_gear_effects(
    gear_slots: &[Option<ItemInstance>],
    gemfinder_shard_amp: bool,
) -> Vec<ItemEffect> {
    gear_slots
        .iter()
        .flatten()
        .flat_map(|item| {
            let mut effects = generate_item_effects(item);
            if gemfinder_shard_amp {
                apply_gemfinder_shard_bonus(item, &mut effects);
            }
            for effect in effects.iter_mut() {
                if effect.effect_type == EffectType::GainAtk {
                    effect.effect_type = EffectType::GainGearAtk;
                }
            }
            effects
        })
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
    let gemfinder_shard_amp = inventory
        .tool
        .as_ref()
        .map(|tool| tool.item_id == GEMFINDER_STAFF_ID)
        .unwrap_or(false);

    // 1. Add tool effects
    if let Some(ref tool) = inventory.tool {
        effects.extend(generate_tool_effects(tool));
    }

    // 2. Add gear effects
    effects.extend(generate_gear_effects(&inventory.gear, gemfinder_shard_amp));

    // 3. Add itemset bonuses
    effects.extend(generate_itemset_effects(inventory));

    effects
}

pub fn generate_annotated_tool_effects(tool: &ItemInstance) -> Vec<AnnotatedItemEffect> {
    annotate_effects(generate_tool_effects(tool), item_source(CombatSourceKind::Tool, &tool.item_id))
}

pub fn generate_annotated_gear_effects(
    gear_slots: &[Option<ItemInstance>],
    gemfinder_shard_amp: bool,
) -> Vec<AnnotatedItemEffect> {
    gear_slots
        .iter()
        .flatten()
        .flat_map(|item| {
            let mut effects = generate_item_effects(item);
            if gemfinder_shard_amp {
                apply_gemfinder_shard_bonus(item, &mut effects);
            }
            for effect in effects.iter_mut() {
                if effect.effect_type == EffectType::GainAtk {
                    effect.effect_type = EffectType::GainGearAtk;
                }
            }
            annotate_effects(effects, item_source(CombatSourceKind::Gear, &item.item_id))
        })
        .collect()
}

pub fn generate_annotated_itemset_effects(inventory: &PlayerInventory) -> Vec<AnnotatedItemEffect> {
    get_active_itemsets(inventory)
        .into_iter()
        .flat_map(|set| {
            annotate_effects(
                set.bonus_effect
                    .iter()
                    .map(|effect_def| effect_def.to_item_effect(Tier::I))
                    .collect(),
                itemset_source(set.id),
            )
        })
        .collect()
}

pub fn generate_annotated_combat_effects(inventory: &PlayerInventory) -> Vec<AnnotatedItemEffect> {
    let mut effects = Vec::new();
    let gemfinder_shard_amp = inventory
        .tool
        .as_ref()
        .map(|tool| tool.item_id == GEMFINDER_STAFF_ID)
        .unwrap_or(false);

    if let Some(ref tool) = inventory.tool {
        effects.extend(generate_annotated_tool_effects(tool));
    }

    effects.extend(generate_annotated_gear_effects(
        &inventory.gear,
        gemfinder_shard_amp,
    ));
    effects.extend(generate_annotated_itemset_effects(inventory));
    effects
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::prelude::Pubkey;
    use combat_system::TriggerType;

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
    fn test_tool_effects_with_tier_scaling() {
        // T-FR-01 (Rime Pike): +1/2/3 ATK, OnHit 1 Chill (flat, per GDD)
        let tool = ItemInstance::new(*b"T-FR-01\0", Tier::II);
        let effects = generate_tool_effects(&tool);

        // Should have 3 effects (ATK, Chill, and chilled-target bonus damage)
        assert_eq!(effects.len(), 3);

        // Check ATK bonus uses Tier II value (2)
        let atk_effect = effects
            .iter()
            .find(|e| e.effect_type == EffectType::GainAtk);
        assert!(atk_effect.is_some());
        assert_eq!(atk_effect.unwrap().value, 2);

        // Check Chill is flat 1 (per GDD: "On Hit (once/turn): apply 1 Chill")
        let chill_effect = effects
            .iter()
            .find(|e| e.effect_type == EffectType::ApplyChill);
        assert!(chill_effect.is_some());
        assert_eq!(chill_effect.unwrap().value, 1);
    }

    #[test]
    fn test_tool_oil_effects() {
        let mut tool = ItemInstance::new(*b"T-FR-01\0", Tier::I);
        tool.apply_oil(ToolOilModification::PlusAtk);
        tool.apply_oil(ToolOilModification::PlusSpd);
        tool.apply_oil(ToolOilModification::PlusArm);

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

        let arm_effects: Vec<_> = effects
            .iter()
            .filter(|e| e.effect_type == EffectType::GainArmor)
            .collect();
        assert_eq!(arm_effects.len(), 1); // Oil +1 ARM
        assert_eq!(arm_effects[0].value, 1);
    }

    #[test]
    fn test_gear_effects_accumulation() {
        let mut inventory = make_inventory();

        // G-ST-01 (Miner Helmet): +3/6/9 ARM
        inventory.gear[0] = Some(ItemInstance::new(*b"G-ST-01\0", Tier::I));
        // G-ST-02 (Work Vest): +2/4/6 ARM, +2/3/4 Heal
        inventory.gear[1] = Some(ItemInstance::new(*b"G-ST-02\0", Tier::I));

        let effects = generate_gear_effects(&inventory.gear, false);

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
        // - 3 from tool (ATK, Chill, chilled-target bonus damage)
        // - 1 from G-ST-01 (ARM)
        // - 2 from G-ST-02 (ARM, Heal)
        // - 1 DIG and 1 SPD from G-SC-01 (Miner Boots now gives SPD)
        // - 2 from Union Standard (ARM, DIG)
        // Total: 10 effects
        assert_eq!(effects.len(), 10);
    }

    #[test]
    fn test_empty_inventory() {
        let inventory = make_inventory();
        let effects = generate_combat_effects(&inventory);
        assert!(effects.is_empty());
    }

    #[test]
    fn test_gemfinder_staff_amplifies_shard_outputs() {
        let mut inventory = make_inventory();
        inventory.tool = Some(ItemInstance::new(*b"T-GR-02\0", Tier::I));
        inventory.gear[0] = Some(ItemInstance::new(*b"G-GR-05\0", Tier::I)); // Emerald
        inventory.gear[1] = Some(ItemInstance::new(*b"G-GR-06\0", Tier::I)); // Ruby
        inventory.gear[2] = Some(ItemInstance::new(*b"G-GR-07\0", Tier::I)); // Sapphire
        inventory.gear[3] = Some(ItemInstance::new(*b"G-GR-08\0", Tier::I)); // Citrine

        let effects = generate_combat_effects(&inventory);

        let emerald = effects
            .iter()
            .find(|e| e.trigger == TriggerType::EveryOtherTurnFirstHit && e.effect_type == EffectType::Heal)
            .unwrap();
        assert_eq!(emerald.value, 3, "Emerald shard should heal 3 at T1 with Gemfinder");

        let ruby = effects
            .iter()
            .find(|e| {
                e.trigger == TriggerType::EveryOtherTurnFirstHit
                    && e.effect_type == EffectType::DealNonWeaponDamage
            })
            .unwrap();
        assert_eq!(ruby.value, 2, "Ruby shard should deal 2 at T1 with Gemfinder");

        let sapphire = effects
            .iter()
            .find(|e| {
                e.trigger == TriggerType::EveryOtherTurnFirstHit
                    && e.effect_type == EffectType::GainArmor
                    && e.value >= 3
            })
            .unwrap();
        assert_eq!(
            sapphire.value, 3,
            "Sapphire shard should gain 3 armor at T1 with Gemfinder"
        );

        let citrine_gold = effects
            .iter()
            .find(|e| {
                e.trigger == TriggerType::EveryOtherTurnFirstHit
                    && e.effect_type == EffectType::GainGold
            })
            .unwrap();
        assert_eq!(
            citrine_gold.value, 3,
            "Citrine shard should gain 3 gold at T1 with Gemfinder"
        );
    }
}
