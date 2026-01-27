//! Player Inventory Program for Dungeons & Moles
//!
//! This program manages player item inventories, including:
//! - Item registry (80 items with effects)
//! - Inventory management (1 Tool + up to 8 Gear)
//! - Item tier upgrades via fusion
//! - Itemset bonus detection (12 itemsets)
//! - Item offer generation for POIs
//! - Combat effect integration

use anchor_lang::prelude::*;

pub mod constants;
pub mod effects;
pub mod errors;
pub mod fusion;
pub mod items;
pub mod itemsets;
pub mod offers;
pub mod state;

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

declare_id!("8vNWjVvz8ZZvPLCKhNJcJH6vxCqNKJgq3hLYAqYdQmQZ");

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

    /// Equips a tool item. If a tool is already equipped, it is replaced.
    /// The item must be a valid Tool type.
    pub fn equip_tool(ctx: Context<EquipTool>, item_id: [u8; 8], tier: Tier) -> Result<()> {
        // Validate item exists and is a tool
        let item_def = get_item(&item_id).ok_or(InventoryError::InvalidItemId)?;
        require!(
            item_def.item_type == ItemType::Tool,
            InventoryError::WrongItemType
        );

        let inventory = &mut ctx.accounts.inventory;
        inventory.tool = Some(ItemInstance::new(item_id, tier));

        emit!(ItemEquipped {
            player: ctx.accounts.player.key(),
            item_id,
            tier,
            slot: "tool".to_string(),
        });

        Ok(())
    }

    /// Equips a gear item in an available slot.
    /// Fails if no gear slots are available.
    pub fn equip_gear(ctx: Context<EquipGear>, item_id: [u8; 8], tier: Tier) -> Result<()> {
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

        emit!(ItemEquipped {
            player: ctx.accounts.player.key(),
            item_id,
            tier,
            slot: format!("gear[{}]", slot_index),
        });

        Ok(())
    }

    /// Removes a gear item from a specific slot.
    /// The item is discarded (not stored elsewhere).
    pub fn unequip_gear(ctx: Context<UnequipGear>, slot_index: u8) -> Result<()> {
        let inventory = &mut ctx.accounts.inventory;

        require!(
            (slot_index as usize) < MAX_GEAR_SLOTS as usize,
            InventoryError::InvalidSlotIndex
        );

        let item = inventory.gear[slot_index as usize]
            .take()
            .ok_or(InventoryError::SlotEmpty)?;

        emit!(ItemUnequipped {
            player: ctx.accounts.player.key(),
            item_id: item.item_id,
            slot_index,
        });

        Ok(())
    }

    /// Fuses two identical items to produce one item at the next tier.
    /// Both items must have the same item_id and tier.
    /// For gear items, specify two gear slot indices.
    pub fn fuse_items(ctx: Context<FuseItems>, slot_a: u8, slot_b: u8) -> Result<()> {
        let inventory = &mut ctx.accounts.inventory;

        // Validate slot indices
        require!(
            (slot_a as usize) < MAX_GEAR_SLOTS as usize
                && (slot_b as usize) < MAX_GEAR_SLOTS as usize,
            InventoryError::InvalidSlotIndex
        );
        require!(slot_a != slot_b, InventoryError::FusionMismatch);

        // Get items from slots
        let item_a = inventory.gear[slot_a as usize]
            .as_ref()
            .ok_or(InventoryError::SlotEmpty)?;
        let item_b = inventory.gear[slot_b as usize]
            .as_ref()
            .ok_or(InventoryError::SlotEmpty)?;

        // Validate fusion
        validate_fusion(item_a, item_b)?;

        // Execute fusion
        let old_tier = item_a.tier;
        let new_tier = execute_fusion(old_tier)?;
        let item_id = item_a.item_id;

        // Update inventory: upgrade first slot, clear second slot
        inventory.gear[slot_a as usize] = Some(ItemInstance::new(item_id, new_tier));
        inventory.gear[slot_b as usize] = None;

        emit!(ItemFused {
            player: ctx.accounts.player.key(),
            item_id,
            old_tier,
            new_tier,
        });

        Ok(())
    }

    /// Applies a Tool Oil modification to the equipped tool.
    /// Each modification (+ATK, +SPD, +DIG) can only be applied once per tool.
    pub fn apply_tool_oil(
        ctx: Context<ApplyToolOil>,
        modification: ToolOilModification,
    ) -> Result<()> {
        let inventory = &mut ctx.accounts.inventory;

        let tool = inventory
            .tool
            .as_mut()
            .ok_or(InventoryError::NoToolEquipped)?;

        require!(
            !tool.has_oil(modification),
            InventoryError::ToolOilAlreadyApplied
        );

        tool.apply_oil(modification);

        emit!(ToolOilApplied {
            player: ctx.accounts.player.key(),
            modification,
        });

        Ok(())
    }

    /// Increases gear slot capacity after defeating a boss.
    /// Week 1 boss: 4 -> 6 slots. Week 2 boss: 6 -> 8 slots.
    pub fn expand_gear_slots(ctx: Context<ExpandGearSlots>) -> Result<()> {
        let inventory = &mut ctx.accounts.inventory;

        let old_capacity = inventory.gear_slot_capacity;

        // Expand slots (this validates and returns error if already max)
        inventory.expand_slots()?;

        let new_capacity = inventory.gear_slot_capacity;

        emit!(GearSlotsExpanded {
            player: ctx.accounts.player.key(),
            old_capacity,
            new_capacity,
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
}

// =============================================================================
// Account Contexts
// =============================================================================

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
pub struct UnequipGear<'info> {
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
pub struct InventoryClosed {
    pub player: Pubkey,
}
