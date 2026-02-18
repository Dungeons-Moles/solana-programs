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
use ephemeral_rollups_sdk::anchor::{commit, delegate, ephemeral};
use ephemeral_rollups_sdk::cpi::DelegateConfig;
use ephemeral_rollups_sdk::ephem::commit_and_undelegate_accounts;

pub mod constants;
pub mod effects;
pub mod errors;
pub mod fusion;
pub mod items;
pub mod itemsets;
pub mod offers;
pub mod state;

use combat_system::{EffectType, TriggerType};
use constants::MAX_GEAR_SLOTS;
use effects::generate_combat_effects;
use errors::InventoryError;
use fusion::{execute_fusion, validate_fusion};
use items::{get_item, BASIC_PICKAXE};
use offers::generate_item_offer;
use state::{
    ItemEffect, ItemInstance, ItemOffer, ItemTag, ItemType, PlayerInventory, PoiType, Tier,
    ToolOilModification,
};

declare_id!("5BtqiWegvVAgEnTRUofB9oUoQvPztYqSkMPwRpYQacP8");

/// Seed for inventory_authority PDA used for CPI calls to other programs
pub const INVENTORY_AUTHORITY_SEED: &[u8] = b"inventory_authority";

/// POI system program ID for authorized equip operations via CPI
/// Derived from "6E27r1Cyo2CNPvtRsonn3uHUAdznS3cMXEBX4HRbfBQY"
pub const POI_SYSTEM_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    77, 160, 63, 209, 182, 56, 149, 181, 2, 195, 173, 95, 65, 136, 88, 122, 235, 166, 235, 216,
    241, 107, 2, 35, 185, 14, 177, 21, 150, 103, 215, 77,
]);

/// Gameplay state program ID for HP modification CPI
/// Derived from "5VAaGSSoBP4UEt3RL2EXvDwpeDxAXMndsJn7QX96nc4n"
pub const GAMEPLAY_STATE_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    66, 165, 213, 208, 125, 103, 44, 88, 115, 217, 192, 197, 1, 117, 7, 170, 78, 32, 208, 143, 119,
    94, 47, 124, 229, 196, 47, 149, 235, 227, 237, 31,
]);
pub const LOCAL_ER_VALIDATOR: Pubkey = pubkey!("mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev");

fn local_delegate_config() -> DelegateConfig {
    DelegateConfig {
        validator: Some(LOCAL_ER_VALIDATOR),
        ..DelegateConfig::default()
    }
}

#[ephemeral]
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
    pub fn delegate_inventory(ctx: Context<DelegateInventory>) -> Result<()> {
        let session_key = ctx.accounts.session.key();
        let (expected_inventory, _) =
            Pubkey::find_program_address(&[b"inventory", session_key.as_ref()], &crate::ID);
        require_keys_eq!(
            ctx.accounts.inventory.key(),
            expected_inventory,
            InventoryError::Unauthorized
        );
        let inventory_seeds: &[&[u8]] = &[b"inventory", session_key.as_ref()];
        ctx.accounts.delegate_inventory(
            &ctx.accounts.player,
            inventory_seeds,
            local_delegate_config(),
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
        require_keys_eq!(
            ctx.accounts.inventory.session,
            session_key,
            InventoryError::Unauthorized
        );
        require_keys_eq!(
            ctx.accounts.inventory.player,
            ctx.accounts.session_signer.key(),
            InventoryError::Unauthorized
        );

        let inventory_info = ctx.accounts.inventory.to_account_info();
        commit_and_undelegate_accounts(
            &ctx.accounts.session_signer.to_account_info(),
            vec![&inventory_info],
            &ctx.accounts.magic_context,
            &ctx.accounts.magic_program.to_account_info(),
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

        // Calculate HP bonus of the removed item (MaxHp effect like Work Vest)
        let hp_bonus: i16 = if let Some(item_def) = get_item(&item.item_id) {
            item_def
                .effects
                .iter()
                .filter(|e| {
                    e.trigger == TriggerType::BattleStart && e.effect_type == EffectType::MaxHp
                })
                .map(|e| e.value_for_tier(item.tier))
                .sum()
        } else {
            0
        };

        // If there was an HP bonus, call gameplay-state to remove it
        if hp_bonus > 0 {
            // Calculate new max HP after removing this item
            let new_max_hp = calculate_max_hp_from_inventory(inventory);
            remove_hp_bonus_cpi(
                &ctx.accounts.game_state,
                &ctx.accounts.inventory_authority,
                &ctx.accounts.gameplay_state_program,
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
                &ctx.accounts.gameplay_state_program,
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
        let old_hp_bonus: i16 = if let Some(ref old_tool) = inventory.tool {
            if let Some(old_def) = get_item(&old_tool.item_id) {
                old_def
                    .effects
                    .iter()
                    .filter(|e| {
                        e.trigger == TriggerType::BattleStart && e.effect_type == EffectType::MaxHp
                    })
                    .map(|e| e.value_for_tier(old_tool.tier))
                    .sum()
            } else {
                0
            }
        } else {
            0
        };

        // Equip new tool
        inventory.tool = Some(ItemInstance::new(item_id, tier));

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
                &ctx.accounts.gameplay_state_program,
                ctx.bumps.inventory_authority,
                hp_delta,
            )?;
        } else if hp_delta < 0 {
            // Net HP loss - need to calculate new max HP and cap current HP
            let new_max_hp = calculate_max_hp_from_inventory(inventory);
            remove_hp_bonus_cpi(
                &ctx.accounts.game_state,
                &ctx.accounts.inventory_authority,
                &ctx.accounts.gameplay_state_program,
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
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Calculate max HP from inventory.
/// Base HP (10) + sum of all BattleStart MaxHp effects from equipped items (e.g., Work Vest).
fn calculate_max_hp_from_inventory(inventory: &PlayerInventory) -> i16 {
    const BASE_HP: i16 = 10;
    let effects = generate_combat_effects(inventory);

    let hp_bonus: i16 = effects
        .iter()
        .filter(|e| e.trigger == TriggerType::BattleStart && e.effect_type == EffectType::MaxHp)
        .map(|e| e.value)
        .sum();

    BASE_HP.saturating_add(hp_bonus)
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
    _gameplay_state_program: &AccountInfo<'info>,
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
    _gameplay_state_program: &AccountInfo<'info>,
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

#[delegate]
#[derive(Accounts)]
pub struct DelegateInventory<'info> {
    #[account(mut, del)]
    /// CHECK: PDA is validated in handler.
    pub inventory: AccountInfo<'info>,
    /// CHECK: Session PDA owned by session-manager; used only for seed derivation.
    pub session: UncheckedAccount<'info>,
    pub player: Signer<'info>,
}

#[commit]
#[derive(Accounts)]
pub struct UndelegateInventory<'info> {
    #[account(mut)]
    pub inventory: Account<'info, PlayerInventory>,
    /// CHECK: Session PDA used only for deterministic PDA validation.
    pub session: UncheckedAccount<'info>,
    pub session_signer: Signer<'info>,
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
    /// CHECK: Session account from session-manager program
    pub session: AccountInfo<'info>,

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
    pub game_state: AccountInfo<'info>,

    /// Inventory authority PDA for signing CPI calls to gameplay-state
    /// CHECK: This is a PDA derived from player-inventory program, validated by seeds
    #[account(
        seeds = [INVENTORY_AUTHORITY_SEED],
        bump,
    )]
    pub inventory_authority: AccountInfo<'info>,

    /// Gameplay state program for HP modification CPI
    /// CHECK: Validated by program ID constant
    #[account(address = GAMEPLAY_STATE_PROGRAM_ID)]
    pub gameplay_state_program: AccountInfo<'info>,

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
    pub game_state: AccountInfo<'info>,

    /// Inventory authority PDA for signing CPI calls to gameplay-state
    /// CHECK: This is a PDA derived from player-inventory program, validated by seeds
    #[account(
        seeds = [INVENTORY_AUTHORITY_SEED],
        bump,
    )]
    pub inventory_authority: AccountInfo<'info>,

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
    pub gameplay_state_program: AccountInfo<'info>,
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
    pub game_state: AccountInfo<'info>,

    /// Inventory authority PDA for signing CPI calls to gameplay-state
    /// CHECK: This is a PDA derived from player-inventory program, validated by seeds
    #[account(
        seeds = [INVENTORY_AUTHORITY_SEED],
        bump,
    )]
    pub inventory_authority: AccountInfo<'info>,

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
    pub gameplay_state_program: AccountInfo<'info>,
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
    pub game_state: AccountInfo<'info>,

    /// Inventory authority PDA for signing CPI calls to gameplay-state
    /// CHECK: This is a PDA derived from player-inventory program, validated by seeds
    #[account(
        seeds = [INVENTORY_AUTHORITY_SEED],
        bump,
    )]
    pub inventory_authority: AccountInfo<'info>,

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
    pub gameplay_state_program: AccountInfo<'info>,
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
