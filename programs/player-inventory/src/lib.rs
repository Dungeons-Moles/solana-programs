//! Player Inventory Program for Dungeons & Moles
//!
//! This program manages player item inventories, including:
//! - Item registry (93 items with effects)
//! - Inventory management (1 Tool + up to 8 Gear)
//! - Item tier upgrades via fusion
//! - Itemset bonus detection (12 itemsets)
//! - Item offer generation for POIs
//! - Combat effect integration

use anchor_lang::prelude::*;
use er_compat::DelegateConfig;

pub mod constants;
pub mod effects;
pub mod errors;
pub mod fusion;
pub mod items;
pub mod itemsets;
pub mod nft_items;
pub mod offers;
pub mod relics;
pub mod state;

use combat_system::{EffectType, TriggerType};
use constants::MAX_GEAR_SLOTS;
use effects::generate_combat_effects;
use errors::InventoryError;
use fusion::{execute_fusion, validate_fusion};
use items::{get_item, BASIC_PICKAXE};
use offers::generate_item_offer;
use relics::get_relic_item;
use state::{
    ItemEffect, ItemInstance, ItemOffer, ItemTag, ItemType, PlayerInventory, PoiType, Tier,
    ToolOilModification,
};

declare_id!("GrXaTaf7wZ74mTaWQ9QSUPAKG6M3Sf4xaZjNytTLa8yC");

/// Seed for inventory_authority PDA used for CPI calls to other programs
pub const INVENTORY_AUTHORITY_SEED: &[u8] = b"inventory_authority";

/// POI system program ID for authorized equip operations via CPI
pub const POI_SYSTEM_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0x65, 0xd1, 0x76, 0xb1, 0x94, 0xe7, 0xc4, 0x89, 0xa6, 0x09, 0xbd, 0xa7, 0x8c, 0x0c, 0x0a, 0xe6,
    0xf6, 0xae, 0xd1, 0x4e, 0xf0, 0xba, 0xe9, 0x21, 0xbb, 0xde, 0x72, 0x90, 0xf1, 0x04, 0xcc, 0xf9,
]);

/// Gameplay state program ID for HP modification CPI
pub const GAMEPLAY_STATE_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0x2a, 0x85, 0x94, 0xcf, 0xca, 0x5f, 0x00, 0x45, 0x30, 0xce, 0x64, 0xd1, 0x54, 0x94, 0x6b, 0x36,
    0xcb, 0xd4, 0x94, 0x56, 0x16, 0x97, 0xa1, 0x82, 0x0d, 0x72, 0x1b, 0x7e, 0x89, 0xb7, 0xbf, 0x7e,
]);

/// Session manager program ID for session key rotation CPI
pub const SESSION_MANAGER_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0xb0, 0x1c, 0x9d, 0x6a, 0x40, 0xc6, 0xe0, 0xa9, 0xe4, 0xaf, 0xa9, 0xa9, 0xd9, 0xad, 0x02, 0x15,
    0x89, 0xbd, 0xf1, 0x36, 0x79, 0x88, 0x02, 0x94, 0xc2, 0x24, 0x9f, 0xd9, 0xa4, 0x21, 0xd7, 0x39,
]);

pub const SESSION_MANAGER_AUTHORITY_SEED: &[u8] = b"session_manager_authority";
fn local_delegate_config(validator: Option<Pubkey>) -> DelegateConfig {
    DelegateConfig {
        // Transmute Pubkey between versions (identical 32-byte layout)
        validator: validator.map(|v| unsafe { std::mem::transmute(v) }),
        ..DelegateConfig::default()
    }
}

#[program]
pub mod player_inventory {
    use super::*;

    /// Creates a new PlayerInventory account for a session.
    /// Initializes with 4 gear slots and Basic Pickaxe equipped.
    /// Each session has its own inventory, ensuring clean state per run.
    pub fn initialize_inventory(ctx: Context<InitializeInventory>) -> Result<()> {
        let inventory = &mut ctx.accounts.inventory;
        inventory.init(
            ctx.accounts.session.key(),
            ctx.accounts.player.key(),
            ctx.bumps.inventory,
        );

        // Auto-equip Basic Pickaxe as the starter tool
        inventory.tool = Some(ItemInstance::new(*BASIC_PICKAXE.id, Tier::I));

        emit!(InventoryInitialized {
            player: ctx.accounts.player.key(),
            session: ctx.accounts.session.key(),
        });

        Ok(())
    }

    /// Delegates inventory PDA to MagicBlock from player-inventory (its owner program).
    pub fn delegate_inventory(
        ctx: Context<DelegateInventory>,
        validator: Option<Pubkey>,
    ) -> Result<()> {
        let session_key = ctx.accounts.session.key();
        let (expected_inventory, _) =
            Pubkey::find_program_address(&[b"inventory", session_key.as_ref()], &crate::ID);
        require_keys_eq!(
            ctx.accounts.inventory.key(),
            expected_inventory,
            InventoryError::Unauthorized
        );
        let inventory_seeds: &[&[u8]] = &[b"inventory", session_key.as_ref()];
        er_compat::delegate_account(
            &ctx.accounts.player.to_account_info(),
            &ctx.accounts.inventory,
            &ctx.accounts.owner_program,
            &ctx.accounts.buffer_inventory,
            &ctx.accounts.delegation_record_inventory,
            &ctx.accounts.delegation_metadata_inventory,
            &ctx.accounts.delegation_program,
            &ctx.accounts.system_program.to_account_info(),
            inventory_seeds,
            local_delegate_config(validator),
        )?;
        Ok(())
    }

    /// Commits and undelegates inventory PDA from ER back to base layer.
    pub fn undelegate_inventory(ctx: Context<UndelegateInventory>) -> Result<()> {
        let session_key = ctx.accounts.session.key();
        let (expected_inventory, _) =
            Pubkey::find_program_address(&[b"inventory", session_key.as_ref()], &crate::ID);
        require_keys_eq!(
            ctx.accounts.inventory.key(),
            expected_inventory,
            InventoryError::Unauthorized
        );
        let inventory = read_inventory(&ctx.accounts.inventory)?;
        require_keys_eq!(inventory.session, session_key, InventoryError::Unauthorized);
        require_keys_eq!(
            inventory.player,
            ctx.accounts.session_signer.key(),
            InventoryError::Unauthorized
        );

        let inventory_info = ctx.accounts.inventory.to_account_info();
        er_compat::commit_and_undelegate(
            ctx.accounts.session_signer.to_account_info(),
            ctx.accounts.magic_context.to_account_info(),
            ctx.accounts.magic_program.to_account_info(),
            &[inventory_info],
        )?;
        Ok(())
    }

    /// DEPRECATED: Use poi-system interactions (interact_pick_item, shop_purchase) instead.
    ///
    /// This instruction is retained for backwards compatibility but does NOT update HP
    /// when equipping items with +HP effects. Items equipped via this instruction will
    /// not grant their HP bonus until the player takes damage and heals.
    ///
    /// For proper HP synchronization, use:
    /// - poi-system::interact_pick_item for POI item picks
    /// - poi-system::shop_purchase for shop purchases
    ///
    /// These call equip_tool_authorized via CPI which properly handles HP updates.
    pub fn equip_tool(ctx: Context<EquipTool>, item_id: [u8; 8], tier: Tier) -> Result<()> {
        let _ = (ctx, item_id, tier);
        err!(InventoryError::DirectMutationDisabled)
    }

    /// DEPRECATED: Use poi-system interactions (interact_pick_item, shop_purchase) instead.
    ///
    /// This instruction is retained for backwards compatibility but does NOT update HP
    /// when equipping items with +HP effects. Items equipped via this instruction will
    /// not grant their HP bonus until the player takes damage and heals.
    ///
    /// For proper HP synchronization, use:
    /// - poi-system::interact_pick_item for POI item picks
    /// - poi-system::shop_purchase for shop purchases
    ///
    /// These call equip_gear_authorized via CPI which properly handles HP updates.
    pub fn equip_gear(ctx: Context<EquipGear>, item_id: [u8; 8], tier: Tier) -> Result<()> {
        let _ = (ctx, item_id, tier);
        err!(InventoryError::DirectMutationDisabled)
    }

    /// Removes a gear item from a specific slot.
    /// The item is discarded (not stored elsewhere).
    ///
    /// If the gear provides +HP (BattleStart Heal effect), this instruction
    /// also calls gameplay-state to remove the HP bonus and cap current HP
    /// at the new max.
    pub fn unequip_gear(ctx: Context<UnequipGear>, slot_index: u8) -> Result<()> {
        let _ = (ctx, slot_index);
        err!(InventoryError::DirectMutationDisabled)
    }

    /// Removes a gear item from a specific slot, authorized by poi-system via CPI.
    /// Used by Scrap Chute to atomically remove gear and adjust HP bonuses.
    pub fn unequip_gear_authorized(
        ctx: Context<UnequipGearAuthorized>,
        slot_index: u8,
    ) -> Result<()> {
        let inventory = &mut ctx.accounts.inventory;

        require!(
            (slot_index as usize) < MAX_GEAR_SLOTS as usize,
            InventoryError::InvalidSlotIndex
        );

        let item = inventory.gear[slot_index as usize]
            .take()
            .ok_or(InventoryError::SlotEmpty)?;
        inventory.gear_relic_assets[slot_index as usize] = None;

        let hp_bonus = item_hp_bonus(&item);

        // If there was an HP bonus, call gameplay-state to remove it
        if hp_bonus > 0 {
            let campaign_level = read_campaign_level(&ctx.accounts.game_state)?;
            let new_max_hp = calculate_max_hp_from_inventory(inventory, base_hp(campaign_level));
            remove_hp_bonus_cpi(
                &ctx.accounts.game_state,
                &ctx.accounts.inventory_authority,
                ctx.bumps.inventory_authority,
                hp_bonus,
                new_max_hp,
            )?;
        }

        emit!(ItemUnequipped {
            player: inventory.player,
            item_id: item.item_id,
            slot_index,
        });

        Ok(())
    }

    /// Fuses two identical items to produce one item at the next tier.
    /// Both items must have the same item_id and tier.
    /// For gear items, specify two gear slot indices.
    pub fn fuse_items(ctx: Context<FuseItems>, slot_a: u8, slot_b: u8) -> Result<()> {
        let _ = (ctx, slot_a, slot_b);
        err!(InventoryError::DirectMutationDisabled)
    }

    /// Upgrades the equipped tool to the next tier while preserving Tool Oil flags.
    ///
    /// Used by poi-system Rusty Anvil interaction for atomic tier upgrades.
    pub fn upgrade_tool_tier(
        ctx: Context<UpgradeToolTier>,
        item_id: [u8; 8],
        current_tier: Tier,
    ) -> Result<()> {
        let _ = (ctx, item_id, current_tier);
        err!(InventoryError::DirectMutationDisabled)
    }

    /// Applies a Tool Oil modification to the equipped tool.
    /// Each modification (+ATK, +SPD, +DIG) can only be applied once per tool.
    pub fn apply_tool_oil(
        ctx: Context<ApplyToolOil>,
        modification: ToolOilModification,
    ) -> Result<()> {
        let _ = (ctx, modification);
        err!(InventoryError::DirectMutationDisabled)
    }

    /// DEPRECATED: Use expand_gear_slots_authorized via gameplay-state instead.
    ///
    /// This instruction is retained for backwards compatibility but should not be
    /// called directly. Gear slots should only expand after boss defeat, which is
    /// validated by gameplay-state before calling expand_gear_slots_authorized.
    pub fn expand_gear_slots(ctx: Context<ExpandGearSlots>) -> Result<()> {
        let _ = ctx;
        err!(InventoryError::DirectMutationDisabled)
    }

    /// Increases gear slot capacity after defeating a boss, authorized by gameplay-state.
    ///
    /// This instruction can only be called via CPI from gameplay-state using
    /// the gameplay_authority PDA as signer. Gameplay-state validates that a boss
    /// was actually defeated before calling this.
    ///
    /// Week 1 boss: 4 -> 6 slots. Week 2 boss: 6 -> 8 slots.
    pub fn expand_gear_slots_authorized(ctx: Context<ExpandGearSlotsAuthorized>) -> Result<()> {
        let inventory = &mut ctx.accounts.inventory;

        let old_capacity = inventory.gear_slot_capacity;

        // Expand slots (this validates and returns error if already max)
        inventory.expand_slots()?;

        let new_capacity = inventory.gear_slot_capacity;

        emit!(GearSlotsExpanded {
            player: inventory.player,
            old_capacity,
            new_capacity,
        });

        Ok(())
    }

    /// Fuses two gear items, authorized by poi-system via CPI (Rune Kiln).
    pub fn fuse_items_authorized(
        ctx: Context<FuseItemsAuthorized>,
        slot_a: u8,
        slot_b: u8,
    ) -> Result<()> {
        let inventory = &mut ctx.accounts.inventory;
        let _item_a = inventory.gear[slot_a as usize].ok_or(InventoryError::SlotEmpty)?;
        let _item_b = inventory.gear[slot_b as usize].ok_or(InventoryError::SlotEmpty)?;
        require!(
            !inventory.gear_slot_is_relic(slot_a as usize)
                && !inventory.gear_slot_is_relic(slot_b as usize),
            InventoryError::RelicModificationNotSupported
        );
        let (item_id, old_tier, new_tier) = fuse_items_internal(inventory, slot_a, slot_b)?;

        emit!(ItemFused {
            player: inventory.player,
            item_id,
            old_tier,
            new_tier,
        });

        Ok(())
    }

    /// Upgrades tool tier, authorized by poi-system via CPI (Rusty Anvil).
    pub fn upgrade_tool_tier_authorized(
        ctx: Context<UpgradeToolTierAuthorized>,
        item_id: [u8; 8],
        current_tier: Tier,
    ) -> Result<()> {
        let inventory = &mut ctx.accounts.inventory;
        let new_tier = upgrade_tool_tier_internal(inventory, item_id, current_tier)?;

        emit!(ToolTierUpgraded {
            player: inventory.player,
            item_id,
            old_tier: current_tier,
            new_tier,
        });

        Ok(())
    }

    /// Applies tool oil, authorized by poi-system via CPI (Tool Oil Rack).
    pub fn apply_tool_oil_authorized(
        ctx: Context<ApplyToolOilAuthorized>,
        modification: ToolOilModification,
    ) -> Result<()> {
        let inventory = &mut ctx.accounts.inventory;
        if inventory.tool_is_relic() {
            return err!(InventoryError::RelicModificationNotSupported);
        }
        apply_tool_oil_internal(inventory, modification)?;

        emit!(ToolOilApplied {
            player: inventory.player,
            modification,
        });

        Ok(())
    }

    /// Converts equipped items and active itemsets to ItemEffect arrays.
    /// Returns effects for use by the combat system.
    pub fn get_combat_effects(ctx: Context<GetCombatEffects>) -> Result<Vec<ItemEffect>> {
        let inventory = &ctx.accounts.inventory;
        Ok(generate_combat_effects(inventory))
    }

    /// Generates a weighted item offer based on POI type, act, and boss weaknesses.
    /// Uses deterministic seed for reproducibility.
    pub fn generate_offer(
        _ctx: Context<GenerateOffer>,
        poi_type: PoiType,
        act: u8,
        week: u8,
        weakness_tag_1: ItemTag,
        weakness_tag_2: ItemTag,
        seed: u64,
    ) -> Result<ItemOffer> {
        let offer =
            generate_item_offer(poi_type, act, week, (weakness_tag_1, weakness_tag_2), seed)
                .ok_or(InventoryError::InvalidItemId)?;

        Ok(offer)
    }

    /// Closes the PlayerInventory account, returning rent to the player.
    /// Called automatically when a session ends (victory, defeat, or quit)
    /// to ensure fresh inventory for the next session.
    pub fn close_inventory(ctx: Context<CloseInventory>) -> Result<()> {
        emit!(InventoryClosed {
            player: ctx.accounts.inventory.player,
        });

        Ok(())
    }

    /// Processes undelegation of a previously delegated account.
    /// Generated equivalent of what #[ephemeral] macro would produce.
    pub fn process_undelegation(ctx: Context<InitializeAfterUndelegation>, account_seeds: Vec<Vec<u8>>) -> Result<()> {
        er_compat::undelegate_account(
            &ctx.accounts.base_account,
            &crate::id(),
            &ctx.accounts.buffer,
            &ctx.accounts.payer,
            &ctx.accounts.system_program,
            account_seeds,
        )
    }

    /// Rotates the owner (session_signer) on an inventory account.
    /// Called via CPI from session-manager during rotate_session_key.
    /// Only the session_manager_authority PDA can authorize this.
    pub fn rotate_inventory_owner(
        ctx: Context<RotateInventoryOwner>,
        new_owner: Pubkey,
    ) -> Result<()> {
        ctx.accounts.inventory.player = new_owner;
        Ok(())
    }

    /// Equips a gear item, authorized by poi-system via CPI.
    ///
    /// This instruction can only be called via CPI from poi-system using
    /// the poi_authority PDA as signer. Used when picking items from POIs
    /// or purchasing from shops.
    ///
    /// If the gear provides +HP (BattleStart Heal effect), this instruction
    /// also calls gameplay-state to add the HP bonus atomically.
    pub fn equip_gear_authorized(
        ctx: Context<EquipGearAuthorized>,
        item_id: [u8; 8],
        tier: Tier,
    ) -> Result<()> {
        // Validate item exists and is gear
        let item_def = get_item(&item_id).ok_or(InventoryError::InvalidItemId)?;
        require!(
            item_def.item_type == ItemType::Gear,
            InventoryError::WrongItemType
        );

        let inventory = &mut ctx.accounts.inventory;

        // Find empty slot
        let slot_index = inventory
            .find_empty_gear_slot()
            .ok_or(InventoryError::InventoryFull)?;

        inventory.gear[slot_index] = Some(ItemInstance::new(item_id, tier));
        inventory.gear_relic_assets[slot_index] = None;

        // Calculate HP bonus if the item has BattleStart MaxHp effect (e.g., Work Vest)
        let hp_bonus: i16 = item_def
            .effects
            .iter()
            .filter(|e| e.trigger == TriggerType::BattleStart && e.effect_type == EffectType::MaxHp)
            .map(|e| e.value_for_tier(tier))
            .sum();

        // If there's an HP bonus, call gameplay-state to add it
        if hp_bonus > 0 {
            add_hp_bonus_cpi(
                &ctx.accounts.game_state,
                &ctx.accounts.inventory_authority,
                ctx.bumps.inventory_authority,
                hp_bonus,
            )?;
        }

        emit!(ItemEquipped {
            player: inventory.player,
            item_id,
            tier,
            slot: format!("gear[{}]", slot_index),
        });

        Ok(())
    }

    /// Equips a tool item, authorized by poi-system via CPI.
    ///
    /// This instruction can only be called via CPI from poi-system using
    /// the poi_authority PDA as signer. Used when picking tools from POIs
    /// or purchasing from shops.
    ///
    /// If the tool provides +HP (BattleStart Heal effect), this instruction
    /// also calls gameplay-state to add the HP bonus atomically.
    /// If replacing an existing tool with +HP, the old bonus is removed first.
    pub fn equip_tool_authorized(
        ctx: Context<EquipToolAuthorized>,
        item_id: [u8; 8],
        tier: Tier,
    ) -> Result<()> {
        // Validate item exists and is a tool
        let item_def = get_item(&item_id).ok_or(InventoryError::InvalidItemId)?;
        require!(
            item_def.item_type == ItemType::Tool,
            InventoryError::WrongItemType
        );

        let inventory = &mut ctx.accounts.inventory;

        // Calculate HP bonus of old tool (if any) to remove
        let old_hp_bonus = inventory.tool.as_ref().map(item_hp_bonus).unwrap_or(0);

        // Equip new tool
        inventory.tool = Some(ItemInstance::new(item_id, tier));
        inventory.tool_relic_asset = None;

        // Calculate HP bonus of new tool
        let new_hp_bonus: i16 = item_def
            .effects
            .iter()
            .filter(|e| e.trigger == TriggerType::BattleStart && e.effect_type == EffectType::MaxHp)
            .map(|e| e.value_for_tier(tier))
            .sum();

        // Handle HP changes:
        // 1. If old tool had HP bonus and new doesn't, remove it
        // 2. If new tool has HP bonus and old didn't, add it
        // 3. If both have bonuses, remove old and add new (net effect)
        let hp_delta = new_hp_bonus - old_hp_bonus;

        if hp_delta > 0 {
            // Net HP gain
            add_hp_bonus_cpi(
                &ctx.accounts.game_state,
                &ctx.accounts.inventory_authority,
                ctx.bumps.inventory_authority,
                hp_delta,
            )?;
        } else if hp_delta < 0 {
            // Net HP loss - need to calculate new max HP and cap current HP
            let campaign_level = read_campaign_level(&ctx.accounts.game_state)?;
            let new_max_hp = calculate_max_hp_from_inventory(inventory, base_hp(campaign_level));
            remove_hp_bonus_cpi(
                &ctx.accounts.game_state,
                &ctx.accounts.inventory_authority,
                ctx.bumps.inventory_authority,
                -hp_delta, // Convert to positive
                new_max_hp,
            )?;
        }

        emit!(ItemEquipped {
            player: inventory.player,
            item_id,
            tier,
            slot: "tool".to_string(),
        });

        Ok(())
    }

    /// Equips an asset-backed relic item into the player's inventory.
    pub fn equip_relic_authorized(
        ctx: Context<EquipRelicAuthorized>,
        relic_item_id: [u8; 8],
        relic_asset: Pubkey,
    ) -> Result<()> {
        let item_def = get_relic_item(&relic_item_id).ok_or(InventoryError::InvalidItemId)?;

        let inventory = &mut ctx.accounts.inventory;
        match item_def.item_type {
            ItemType::Gear => {
                let slot_index = inventory
                    .find_empty_gear_slot()
                    .ok_or(InventoryError::InventoryFull)?;
                let item = ItemInstance::new(relic_item_id, Tier::I);
                let hp_bonus = item_hp_bonus(&item);
                inventory.gear[slot_index] = Some(item);
                inventory.gear_relic_assets[slot_index] = Some(relic_asset);

                if hp_bonus > 0 {
                    add_hp_bonus_cpi(
                        &ctx.accounts.game_state,
                        &ctx.accounts.inventory_authority,
                        ctx.bumps.inventory_authority,
                        hp_bonus,
                    )?;
                }

                emit!(ItemEquipped {
                    player: inventory.player,
                    item_id: relic_item_id,
                    tier: Tier::I,
                    slot: format!("gear[{}]", slot_index),
                });
            }
            ItemType::Tool => {
                let old_hp_bonus = inventory.tool.as_ref().map(item_hp_bonus).unwrap_or(0);
                inventory.tool = Some(ItemInstance::new(relic_item_id, Tier::I));
                inventory.tool_relic_asset = Some(relic_asset);
                let new_hp_bonus = inventory.tool.as_ref().map(item_hp_bonus).unwrap_or(0);
                let hp_delta = new_hp_bonus - old_hp_bonus;

                if hp_delta > 0 {
                    add_hp_bonus_cpi(
                        &ctx.accounts.game_state,
                        &ctx.accounts.inventory_authority,
                        ctx.bumps.inventory_authority,
                        hp_delta,
                    )?;
                } else if hp_delta < 0 {
                    let campaign_level = read_campaign_level(&ctx.accounts.game_state)?;
                    let new_max_hp = calculate_max_hp_from_inventory(inventory, base_hp(campaign_level));
                    remove_hp_bonus_cpi(
                        &ctx.accounts.game_state,
                        &ctx.accounts.inventory_authority,
                        ctx.bumps.inventory_authority,
                        -hp_delta,
                        new_max_hp,
                    )?;
                }

                emit!(ItemEquipped {
                    player: inventory.player,
                    item_id: relic_item_id,
                    tier: Tier::I,
                    slot: "tool".to_string(),
                });
            }
        }

        Ok(())
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Calculate max HP from inventory.
/// Base HP (10) + sum of all BattleStart MaxHp effects from equipped items (e.g., Work Vest).
/// Byte offset of `campaign_level` (u8) in the GameState account data.
/// 8 (discriminator) + 32*3 (player, session_signer, session) + 4*u8 (pos/map)
/// + i16 (hp) + u8 (gear_slots) + u8 (week) + u8 (phase) + u8 (moves_remaining)
/// + u32 (total_moves) + bool (boss_fight_ready) + u16 (gold) + u8 (bump) = 122
const GAME_STATE_CAMPAIGN_LEVEL_OFFSET: usize = 122;

/// Mirror of gameplay-state's base_hp(campaign_level) function.
fn base_hp(campaign_level: u8) -> i16 {
    match campaign_level {
        1..=9 => 25,
        10..=19 => 22,
        _ => 18,
    }
}

/// Reads campaign_level from raw GameState account data.
fn read_campaign_level(game_state: &AccountInfo) -> Result<u8> {
    let data = game_state.try_borrow_data()?;
    require!(
        data.len() > GAME_STATE_CAMPAIGN_LEVEL_OFFSET,
        InventoryError::InvalidSlotIndex
    );
    Ok(data[GAME_STATE_CAMPAIGN_LEVEL_OFFSET])
}

fn calculate_max_hp_from_inventory(inventory: &PlayerInventory, base: i16) -> i16 {
    let effects = generate_combat_effects(inventory);

    let hp_bonus: i16 = effects
        .iter()
        .filter(|e| e.trigger == TriggerType::BattleStart && e.effect_type == EffectType::MaxHp)
        .map(|e| e.value)
        .sum();

    base.saturating_add(hp_bonus)
}

fn item_hp_bonus(item: &ItemInstance) -> i16 {
    let Some(definition) = get_item(&item.item_id).or_else(|| get_relic_item(&item.item_id)) else {
        return 0;
    };

    definition
        .effects
        .iter()
        .filter(|e| e.trigger == TriggerType::BattleStart && e.effect_type == EffectType::MaxHp)
        .map(|e| e.value_for_tier(item.tier))
        .sum()
}

fn fuse_items_internal(
    inventory: &mut PlayerInventory,
    slot_a: u8,
    slot_b: u8,
) -> Result<([u8; 8], Tier, Tier)> {
    require!(
        (slot_a as usize) < MAX_GEAR_SLOTS as usize && (slot_b as usize) < MAX_GEAR_SLOTS as usize,
        InventoryError::InvalidSlotIndex
    );
    require!(slot_a != slot_b, InventoryError::FusionMismatch);

    let item_a = inventory.gear[slot_a as usize]
        .as_ref()
        .ok_or(InventoryError::SlotEmpty)?;
    let item_b = inventory.gear[slot_b as usize]
        .as_ref()
        .ok_or(InventoryError::SlotEmpty)?;

    validate_fusion(item_a, item_b)?;

    let old_tier = item_a.tier;
    let new_tier = execute_fusion(old_tier)?;
    let item_id = item_a.item_id;

    inventory.gear[slot_a as usize] = Some(ItemInstance::new(item_id, new_tier));
    inventory.gear[slot_b as usize] = None;
    inventory.gear_relic_assets[slot_a as usize] = None;
    inventory.gear_relic_assets[slot_b as usize] = None;

    Ok((item_id, old_tier, new_tier))
}

fn upgrade_tool_tier_internal(
    inventory: &mut PlayerInventory,
    item_id: [u8; 8],
    current_tier: Tier,
) -> Result<Tier> {
    let tool = inventory
        .tool
        .as_mut()
        .ok_or(InventoryError::NoToolEquipped)?;

    require!(tool.item_id == item_id, InventoryError::InvalidItemId);
    require!(tool.tier == current_tier, InventoryError::FusionMismatch);

    let new_tier = current_tier.next().ok_or(InventoryError::AlreadyMaxTier)?;
    tool.tier = new_tier;
    Ok(new_tier)
}

fn apply_tool_oil_internal(
    inventory: &mut PlayerInventory,
    modification: ToolOilModification,
) -> Result<()> {
    let tool = inventory
        .tool
        .as_mut()
        .ok_or(InventoryError::NoToolEquipped)?;

    require!(
        !tool.has_oil(modification),
        InventoryError::ToolOilAlreadyApplied
    );
    tool.apply_oil(modification);
    Ok(())
}

/// CPI call to gameplay-state::add_hp_bonus_authorized
fn add_hp_bonus_cpi<'info>(
    game_state: &AccountInfo<'info>,
    inventory_authority: &AccountInfo<'info>,
    inventory_authority_bump: u8,
    hp_bonus: i16,
) -> Result<()> {
    let signer_seeds: &[&[&[u8]]] = &[&[INVENTORY_AUTHORITY_SEED, &[inventory_authority_bump]]];

    // Build the instruction manually since we can't import gameplay_state::cpi
    // (would create circular dependency)
    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: GAMEPLAY_STATE_PROGRAM_ID,
        accounts: vec![
            anchor_lang::solana_program::instruction::AccountMeta::new(game_state.key(), false),
            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                inventory_authority.key(),
                true,
            ),
        ],
        // Instruction discriminator for "add_hp_bonus_authorized" + hp_bonus (i16)
        data: {
            let mut data = Vec::with_capacity(8 + 2);
            // Anchor discriminator: sha256("global:add_hp_bonus_authorized")[0..8]
            data.extend_from_slice(&[0x35, 0xde, 0x77, 0x05, 0x49, 0xff, 0x06, 0xa8]);
            data.extend_from_slice(&hp_bonus.to_le_bytes());
            data
        },
    };

    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[game_state.clone(), inventory_authority.clone()],
        signer_seeds,
    )?;

    Ok(())
}

/// CPI call to gameplay-state::remove_hp_bonus_authorized
fn remove_hp_bonus_cpi<'info>(
    game_state: &AccountInfo<'info>,
    inventory_authority: &AccountInfo<'info>,
    inventory_authority_bump: u8,
    hp_bonus: i16,
    new_max_hp: i16,
) -> Result<()> {
    let signer_seeds: &[&[&[u8]]] = &[&[INVENTORY_AUTHORITY_SEED, &[inventory_authority_bump]]];

    // Build the instruction manually since we can't import gameplay_state::cpi
    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: GAMEPLAY_STATE_PROGRAM_ID,
        accounts: vec![
            anchor_lang::solana_program::instruction::AccountMeta::new(game_state.key(), false),
            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                inventory_authority.key(),
                true,
            ),
        ],
        // Instruction discriminator for "remove_hp_bonus_authorized" + hp_bonus (i16) + new_max_hp (i16)
        data: {
            let mut data = Vec::with_capacity(8 + 4);
            // Anchor discriminator: sha256("global:remove_hp_bonus_authorized")[0..8]
            data.extend_from_slice(&[0xd1, 0x7e, 0x67, 0xa0, 0x8f, 0xef, 0xad, 0x8c]);
            data.extend_from_slice(&hp_bonus.to_le_bytes());
            data.extend_from_slice(&new_max_hp.to_le_bytes());
            data
        },
    };

    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[game_state.clone(), inventory_authority.clone()],
        signer_seeds,
    )?;

    Ok(())
}

// =============================================================================
// Account Contexts
// =============================================================================

#[derive(Accounts)]
pub struct DelegateInventory<'info> {
    #[account(mut)]
    /// CHECK: PDA is validated in handler.
    pub inventory: UncheckedAccount<'info>,
    /// CHECK: Session PDA used only for seed derivation. Owner not checked because
    /// the session may already be delegated (owned by delegation program) at this point.
    pub session: UncheckedAccount<'info>,
    pub player: Signer<'info>,
    /// CHECK: Buffer for delegation
    #[account(
        mut,
        seeds = [er_compat::DELEGATE_BUFFER_TAG, inventory.key().as_ref()],
        bump,
        seeds::program = crate::id()
    )]
    pub buffer_inventory: UncheckedAccount<'info>,
    /// CHECK: Delegation record
    #[account(
        mut,
        seeds = [er_compat::DELEGATION_RECORD_TAG, inventory.key().as_ref()],
        bump,
        seeds::program = er_compat::DELEGATION_PROGRAM_ID
    )]
    pub delegation_record_inventory: UncheckedAccount<'info>,
    /// CHECK: Delegation metadata
    #[account(
        mut,
        seeds = [er_compat::DELEGATION_METADATA_TAG, inventory.key().as_ref()],
        bump,
        seeds::program = er_compat::DELEGATION_PROGRAM_ID
    )]
    pub delegation_metadata_inventory: UncheckedAccount<'info>,
    /// CHECK: Owner program
    #[account(address = crate::id())]
    pub owner_program: UncheckedAccount<'info>,
    /// CHECK: Delegation program
    #[account(address = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UndelegateInventory<'info> {
    #[account(mut)]
    /// CHECK: PDA is validated and deserialized in handler.
    pub inventory: UncheckedAccount<'info>,
    /// CHECK: Session PDA used only for seed derivation. Owner not checked because
    /// the session may already be delegated (owned by delegation program) at this point.
    pub session: UncheckedAccount<'info>,
    #[account(mut)]
    pub session_signer: Signer<'info>,
    /// CHECK: Magic program
    #[account(address = er_compat::MAGIC_PROGRAM_ID)]
    pub magic_program: UncheckedAccount<'info>,
    /// CHECK: Magic context
    #[account(mut, address = er_compat::MAGIC_CONTEXT_ID)]
    pub magic_context: UncheckedAccount<'info>,
}

/// Context for undelegation processing (replaces #[ephemeral] macro output).
#[derive(Accounts)]
pub struct InitializeAfterUndelegation<'info> {
    /// CHECK: Account being undelegated
    #[account(mut)]
    pub base_account: UncheckedAccount<'info>,
    /// CHECK: Delegation buffer
    pub buffer: UncheckedAccount<'info>,
    /// CHECK: Payer
    #[account(mut)]
    pub payer: UncheckedAccount<'info>,
    /// CHECK: System program
    pub system_program: UncheckedAccount<'info>,
}

fn read_inventory(inventory: &AccountInfo<'_>) -> Result<PlayerInventory> {
    let data = inventory.try_borrow_data()?;
    let mut slice: &[u8] = &data;
    PlayerInventory::try_deserialize(&mut slice).map_err(|_| InventoryError::InvalidSession.into())
}

#[derive(Accounts)]
pub struct InitializeInventory<'info> {
    /// PDA: ["inventory", session.key()] - one inventory per session
    #[account(
        init,
        payer = player,
        space = PlayerInventory::LEN,
        seeds = [b"inventory", session.key().as_ref()],
        bump
    )]
    pub inventory: Account<'info, PlayerInventory>,

    /// The game session this inventory belongs to
    /// CHECK: Session account from session-manager program, owner validated by constraint
    #[account(owner = SESSION_MANAGER_PROGRAM_ID)]
    pub session: UncheckedAccount<'info>,

    /// Player wallet, pays for account creation
    #[account(mut)]
    pub player: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct EquipTool<'info> {
    #[account(
        mut,
        seeds = [b"inventory", inventory.session.as_ref()],
        bump = inventory.bump,
        has_one = player @ InventoryError::Unauthorized
    )]
    pub inventory: Account<'info, PlayerInventory>,

    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct EquipGear<'info> {
    #[account(
        mut,
        seeds = [b"inventory", inventory.session.as_ref()],
        bump = inventory.bump,
        has_one = player @ InventoryError::Unauthorized
    )]
    pub inventory: Account<'info, PlayerInventory>,

    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpgradeToolTier<'info> {
    #[account(
        mut,
        seeds = [b"inventory", inventory.session.as_ref()],
        bump = inventory.bump,
        has_one = player @ InventoryError::Unauthorized
    )]
    pub inventory: Account<'info, PlayerInventory>,

    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpgradeToolTierAuthorized<'info> {
    #[account(
        mut,
        seeds = [b"inventory", inventory.session.as_ref()],
        bump = inventory.bump,
    )]
    pub inventory: Account<'info, PlayerInventory>,

    #[account(
        seeds = [b"poi_authority"],
        bump,
        seeds::program = POI_SYSTEM_PROGRAM_ID,
    )]
    pub poi_authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct UnequipGear<'info> {
    #[account(
        mut,
        seeds = [b"inventory", inventory.session.as_ref()],
        bump = inventory.bump,
        has_one = player @ InventoryError::Unauthorized
    )]
    pub inventory: Account<'info, PlayerInventory>,

    /// Game state for HP bonus removal
    /// CHECK: Validated by gameplay-state program
    #[account(mut)]
    pub game_state: UncheckedAccount<'info>,

    /// Inventory authority PDA for signing CPI calls to gameplay-state
    /// CHECK: This is a PDA derived from player-inventory program, validated by seeds
    #[account(
        seeds = [INVENTORY_AUTHORITY_SEED],
        bump,
    )]
    pub inventory_authority: UncheckedAccount<'info>,

    /// Gameplay state program for HP modification CPI
    /// CHECK: Validated by program ID constant
    #[account(address = GAMEPLAY_STATE_PROGRAM_ID)]
    pub gameplay_state_program: UncheckedAccount<'info>,

    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct UnequipGearAuthorized<'info> {
    #[account(
        mut,
        seeds = [b"inventory", inventory.session.as_ref()],
        bump = inventory.bump,
    )]
    pub inventory: Account<'info, PlayerInventory>,

    /// Game state for HP bonus removal
    /// CHECK: Validated by gameplay-state program
    #[account(mut)]
    pub game_state: UncheckedAccount<'info>,

    /// Inventory authority PDA for signing CPI calls to gameplay-state
    /// CHECK: This is a PDA derived from player-inventory program, validated by seeds
    #[account(
        seeds = [INVENTORY_AUTHORITY_SEED],
        bump,
    )]
    pub inventory_authority: UncheckedAccount<'info>,

    /// POI authority PDA from poi-system that must sign
    #[account(
        seeds = [b"poi_authority"],
        bump,
        seeds::program = POI_SYSTEM_PROGRAM_ID,
    )]
    pub poi_authority: Signer<'info>,

    /// Gameplay state program for HP modification CPI
    /// CHECK: Validated by program ID constant
    #[account(address = GAMEPLAY_STATE_PROGRAM_ID)]
    pub gameplay_state_program: UncheckedAccount<'info>,
}

/// Context for equipping gear via authorized CPI from poi-system.
/// Requires poi_authority PDA from poi-system as signer.
/// Includes game_state for HP bonus updates.
#[derive(Accounts)]
pub struct EquipGearAuthorized<'info> {
    #[account(
        mut,
        seeds = [b"inventory", inventory.session.as_ref()],
        bump = inventory.bump,
    )]
    pub inventory: Account<'info, PlayerInventory>,

    /// Game state for HP bonus updates
    /// CHECK: Validated by gameplay-state program
    #[account(mut)]
    pub game_state: UncheckedAccount<'info>,

    /// Inventory authority PDA for signing CPI calls to gameplay-state
    /// CHECK: This is a PDA derived from player-inventory program, validated by seeds
    #[account(
        seeds = [INVENTORY_AUTHORITY_SEED],
        bump,
    )]
    pub inventory_authority: UncheckedAccount<'info>,

    /// POI authority PDA from poi-system that must sign
    #[account(
        seeds = [b"poi_authority"],
        bump,
        seeds::program = POI_SYSTEM_PROGRAM_ID,
    )]
    pub poi_authority: Signer<'info>,

    /// Gameplay state program for HP modification CPI
    /// CHECK: Validated by program ID constant
    #[account(address = GAMEPLAY_STATE_PROGRAM_ID)]
    pub gameplay_state_program: UncheckedAccount<'info>,
}

/// Context for equipping tool via authorized CPI from poi-system.
/// Requires poi_authority PDA from poi-system as signer.
/// Includes game_state for HP bonus updates when replacing tools with +HP effects.
#[derive(Accounts)]
pub struct EquipToolAuthorized<'info> {
    #[account(
        mut,
        seeds = [b"inventory", inventory.session.as_ref()],
        bump = inventory.bump,
    )]
    pub inventory: Account<'info, PlayerInventory>,

    /// Game state for HP bonus updates
    /// CHECK: Validated by gameplay-state program
    #[account(mut)]
    pub game_state: UncheckedAccount<'info>,

    /// Inventory authority PDA for signing CPI calls to gameplay-state
    /// CHECK: This is a PDA derived from player-inventory program, validated by seeds
    #[account(
        seeds = [INVENTORY_AUTHORITY_SEED],
        bump,
    )]
    pub inventory_authority: UncheckedAccount<'info>,

    /// POI authority PDA from poi-system that must sign
    #[account(
        seeds = [b"poi_authority"],
        bump,
        seeds::program = POI_SYSTEM_PROGRAM_ID,
    )]
    pub poi_authority: Signer<'info>,

    /// Gameplay state program for HP modification CPI
    /// CHECK: Validated by program ID constant
    #[account(address = GAMEPLAY_STATE_PROGRAM_ID)]
    pub gameplay_state_program: UncheckedAccount<'info>,
}

/// Context for equipping relic items via authorized CPI from poi-system.
#[derive(Accounts)]
pub struct EquipRelicAuthorized<'info> {
    #[account(
        mut,
        seeds = [b"inventory", inventory.session.as_ref()],
        bump = inventory.bump,
    )]
    pub inventory: Account<'info, PlayerInventory>,

    /// POI authority PDA from poi-system that must sign
    #[account(
        seeds = [b"poi_authority"],
        bump,
        seeds::program = POI_SYSTEM_PROGRAM_ID,
    )]
    pub poi_authority: Signer<'info>,

    /// CHECK: PDA derived from player-inventory program, validated by seeds.
    #[account(
        seeds = [INVENTORY_AUTHORITY_SEED],
        bump,
    )]
    pub inventory_authority: UncheckedAccount<'info>,

    /// Gameplay state program for HP modification CPI
    /// CHECK: Validated by program ID constant
    #[account(address = GAMEPLAY_STATE_PROGRAM_ID)]
    pub gameplay_state_program: UncheckedAccount<'info>,

    /// Game state for HP bonus updates
    /// CHECK: Validated by gameplay-state program
    #[account(mut)]
    pub game_state: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct FuseItems<'info> {
    #[account(
        mut,
        seeds = [b"inventory", inventory.session.as_ref()],
        bump = inventory.bump,
        has_one = player @ InventoryError::Unauthorized
    )]
    pub inventory: Account<'info, PlayerInventory>,

    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct FuseItemsAuthorized<'info> {
    #[account(
        mut,
        seeds = [b"inventory", inventory.session.as_ref()],
        bump = inventory.bump,
    )]
    pub inventory: Account<'info, PlayerInventory>,

    #[account(
        seeds = [b"poi_authority"],
        bump,
        seeds::program = POI_SYSTEM_PROGRAM_ID,
    )]
    pub poi_authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct ApplyToolOil<'info> {
    #[account(
        mut,
        seeds = [b"inventory", inventory.session.as_ref()],
        bump = inventory.bump,
        has_one = player @ InventoryError::Unauthorized
    )]
    pub inventory: Account<'info, PlayerInventory>,

    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct ApplyToolOilAuthorized<'info> {
    #[account(
        mut,
        seeds = [b"inventory", inventory.session.as_ref()],
        bump = inventory.bump,
    )]
    pub inventory: Account<'info, PlayerInventory>,

    #[account(
        seeds = [b"poi_authority"],
        bump,
        seeds::program = POI_SYSTEM_PROGRAM_ID,
    )]
    pub poi_authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct ExpandGearSlots<'info> {
    #[account(
        mut,
        seeds = [b"inventory", inventory.session.as_ref()],
        bump = inventory.bump,
        has_one = player @ InventoryError::Unauthorized
    )]
    pub inventory: Account<'info, PlayerInventory>,

    pub player: Signer<'info>,
}

/// Context for expanding gear slots via authorized CPI from gameplay-state.
/// Requires gameplay_authority PDA from gameplay-state as signer.
#[derive(Accounts)]
pub struct ExpandGearSlotsAuthorized<'info> {
    #[account(
        mut,
        seeds = [b"inventory", inventory.session.as_ref()],
        bump = inventory.bump,
    )]
    pub inventory: Account<'info, PlayerInventory>,

    /// Gameplay authority PDA from gameplay-state that must sign
    #[account(
        seeds = [b"gameplay_authority"],
        bump,
        seeds::program = GAMEPLAY_STATE_PROGRAM_ID,
    )]
    pub gameplay_authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct GetCombatEffects<'info> {
    #[account(
        seeds = [b"inventory", inventory.session.as_ref()],
        bump = inventory.bump,
        has_one = player @ InventoryError::Unauthorized
    )]
    pub inventory: Account<'info, PlayerInventory>,

    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct GenerateOffer<'info> {
    /// Any signer can generate offers (view function)
    pub signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct CloseInventory<'info> {
    #[account(
        mut,
        seeds = [b"inventory", inventory.session.as_ref()],
        bump = inventory.bump,
        has_one = player @ InventoryError::Unauthorized,
        close = player
    )]
    pub inventory: Account<'info, PlayerInventory>,

    #[account(mut)]
    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct RotateInventoryOwner<'info> {
    #[account(mut)]
    pub inventory: Account<'info, PlayerInventory>,

    #[account(
        seeds = [SESSION_MANAGER_AUTHORITY_SEED],
        bump,
        seeds::program = SESSION_MANAGER_PROGRAM_ID
    )]
    /// Session manager PDA authority (only session-manager can sign)
    pub session_manager_authority: Signer<'info>,
}

// =============================================================================
// Events
// =============================================================================

#[event]
pub struct InventoryInitialized {
    pub player: Pubkey,
    pub session: Pubkey,
}

#[event]
pub struct ItemEquipped {
    pub player: Pubkey,
    pub item_id: [u8; 8],
    pub tier: Tier,
    pub slot: String,
}

#[event]
pub struct ItemUnequipped {
    pub player: Pubkey,
    pub item_id: [u8; 8],
    pub slot_index: u8,
}

#[event]
pub struct ItemFused {
    pub player: Pubkey,
    pub item_id: [u8; 8],
    pub old_tier: Tier,
    pub new_tier: Tier,
}

#[event]
pub struct GearSlotsExpanded {
    pub player: Pubkey,
    pub old_capacity: u8,
    pub new_capacity: u8,
}

#[event]
pub struct ToolOilApplied {
    pub player: Pubkey,
    pub modification: ToolOilModification,
}

#[event]
pub struct ToolTierUpgraded {
    pub player: Pubkey,
    pub item_id: [u8; 8],
    pub old_tier: Tier,
    pub new_tier: Tier,
}

#[event]
pub struct InventoryClosed {
    pub player: Pubkey,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    /// Validates that ADD_HP_BONUS_AUTHORIZED discriminator matches sha256("global:add_hp_bonus_authorized")[..8].
    #[test]
    fn test_add_hp_bonus_authorized_discriminator() {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(b"global:add_hp_bonus_authorized");
        let expected: [u8; 8] = hash[..8].try_into().unwrap();
        let actual: [u8; 8] = [0x35, 0xde, 0x77, 0x05, 0x49, 0xff, 0x06, 0xa8];
        assert_eq!(
            actual, expected,
            "add_hp_bonus_authorized discriminator mismatch: expected {:?}, got {:?}",
            expected, actual
        );
    }

    /// Validates that REMOVE_HP_BONUS_AUTHORIZED discriminator matches sha256("global:remove_hp_bonus_authorized")[..8].
    #[test]
    fn test_remove_hp_bonus_authorized_discriminator() {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(b"global:remove_hp_bonus_authorized");
        let expected: [u8; 8] = hash[..8].try_into().unwrap();
        let actual: [u8; 8] = [0xd1, 0x7e, 0x67, 0xa0, 0x8f, 0xef, 0xad, 0x8c];
        assert_eq!(
            actual, expected,
            "remove_hp_bonus_authorized discriminator mismatch: expected {:?}, got {:?}",
            expected, actual
        );
    }
}
