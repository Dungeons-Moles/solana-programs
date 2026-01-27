//! State types and enums for the player-inventory program.

use anchor_lang::prelude::*;

use crate::constants::{INITIAL_GEAR_SLOTS, MAX_GEAR_SLOTS};

/// Item types - Tool or Gear
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ItemType {
    /// Equipped in tool slot, provides ATK and strikes
    Tool,
    /// Equipped in gear slots, provides various effects
    Gear,
}

/// Item tags - 8 thematic categories plus None for special items
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ItemTag {
    /// No tag (used for special items like Basic Pickaxe)
    None,
    /// Armor, Shrapnel, durability
    Stone,
    /// DIG, multi-strike, mobility
    Scout,
    /// Gold generation, shards
    Greed,
    /// Countdown bombs, non-weapon damage
    Blast,
    /// Chill, SPD manipulation
    Frost,
    /// Armor destruction
    Rust,
    /// Bleed, sustain
    Blood,
    /// SPD, Turn 1 effects
    Tempo,
}

/// Item rarity levels
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Rarity {
    /// Most items, base stats
    Common,
    /// Enhanced effects
    Rare,
    /// Powerful effects
    Heroic,
    /// Build-defining effects
    Mythic,
}

/// Item tier (upgrade level)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Tier {
    /// Base values (index 0 of values array)
    #[default]
    I,
    /// Middle values (index 1)
    II,
    /// Maximum values (index 2)
    III,
}

impl Tier {
    /// Get the array index for this tier's values
    pub fn index(&self) -> usize {
        match self {
            Tier::I => 0,
            Tier::II => 1,
            Tier::III => 2,
        }
    }

    /// Get the next tier (for fusion)
    pub fn next(&self) -> Option<Tier> {
        match self {
            Tier::I => Some(Tier::II),
            Tier::II => Some(Tier::III),
            Tier::III => None,
        }
    }
}

/// Combat-system trigger/effect types shared across gameplay.
pub use combat_system::state::{EffectType, ItemEffect, TriggerType};

/// Effect definition with tier-scaled values (compile-time constant)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct EffectDefinition {
    /// When effect activates
    pub trigger: TriggerType,
    /// What the effect does
    pub effect_type: EffectType,
    /// Whether effect is limited to once per turn
    pub once_per_turn: bool,
    /// Values for Tier I, II, III
    pub values: [i16; 3],
}

impl EffectDefinition {
    /// Create a new EffectDefinition
    pub const fn new(
        trigger: TriggerType,
        effect_type: EffectType,
        once_per_turn: bool,
        values: [i16; 3],
    ) -> Self {
        Self {
            trigger,
            effect_type,
            once_per_turn,
            values,
        }
    }

    /// Get the value for a specific tier
    pub fn value_for_tier(&self, tier: Tier) -> i16 {
        self.values[tier.index()]
    }

    /// Convert to ItemEffect for combat system
    pub fn to_item_effect(&self, tier: Tier) -> ItemEffect {
        ItemEffect {
            trigger: self.trigger,
            once_per_turn: self.once_per_turn,
            effect_type: self.effect_type,
            value: self.value_for_tier(tier),
        }
    }
}

/// Create a stat bonus effect (BattleStart trigger)
pub fn stat_bonus(effect_type: EffectType, value: i16) -> ItemEffect {
    ItemEffect {
        trigger: TriggerType::BattleStart,
        once_per_turn: false,
        effect_type,
        value,
    }
}

/// Tool Oil modification types
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToolOilModification {
    /// +1 ATK
    PlusAtk,
    /// +1 SPD
    PlusSpd,
    /// +1 DIG
    PlusDig,
}

impl ToolOilModification {
    /// Get the bit flag for this modification
    pub fn flag(&self) -> u8 {
        match self {
            ToolOilModification::PlusAtk => crate::constants::TOOL_OIL_ATK,
            ToolOilModification::PlusSpd => crate::constants::TOOL_OIL_SPD,
            ToolOilModification::PlusDig => crate::constants::TOOL_OIL_DIG,
        }
    }

    /// Get the effect type for this modification
    pub fn effect_type(&self) -> EffectType {
        match self {
            ToolOilModification::PlusAtk => EffectType::GainAtk,
            ToolOilModification::PlusSpd => EffectType::GainSpd,
            ToolOilModification::PlusDig => EffectType::GainDig,
        }
    }
}

/// A specific item owned by a player
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ItemInstance {
    /// References ItemDefinition ID (e.g., "T-ST-01\0")
    pub item_id: [u8; 8],
    /// Upgrade tier
    pub tier: Tier,
    /// Tool Oil bitflags: 0x01=+ATK, 0x02=+SPD, 0x04=+DIG
    pub tool_oil_flags: u8,
}

impl ItemInstance {
    /// Create a new ItemInstance
    pub fn new(item_id: [u8; 8], tier: Tier) -> Self {
        Self {
            item_id,
            tier,
            tool_oil_flags: 0,
        }
    }

    /// Check if a Tool Oil modification is applied
    pub fn has_oil(&self, modification: ToolOilModification) -> bool {
        self.tool_oil_flags & modification.flag() != 0
    }

    /// Apply a Tool Oil modification
    pub fn apply_oil(&mut self, modification: ToolOilModification) {
        self.tool_oil_flags |= modification.flag();
    }
}

/// Player's equipped items and slot capacity
#[account]
pub struct PlayerInventory {
    /// Owner's wallet
    pub player: Pubkey,
    /// Equipped tool (0 or 1)
    pub tool: Option<ItemInstance>,
    /// Equipped gear (0-8)
    pub gear: [Option<ItemInstance>; 8],
    /// Current gear limit (4, 6, or 8)
    pub gear_slot_capacity: u8,
    /// PDA bump seed
    pub bump: u8,
}

impl PlayerInventory {
    /// Account space calculation
    /// 8 (discriminator) + 32 (player) + 1 + 10 (tool option) + 8 * (1 + 10) (gear array) + 1 + 1
    pub const LEN: usize = 8 + 32 + 1 + 10 + (8 * 11) + 1 + 1;

    /// Initialize a new PlayerInventory
    pub fn init(&mut self, player: Pubkey, bump: u8) {
        self.player = player;
        self.tool = None;
        self.gear = [None; 8];
        self.gear_slot_capacity = INITIAL_GEAR_SLOTS;
        self.bump = bump;
    }

    /// Count currently equipped gear items
    pub fn gear_count(&self) -> u8 {
        self.gear.iter().filter(|g| g.is_some()).count() as u8
    }

    /// Find first empty gear slot
    pub fn find_empty_gear_slot(&self) -> Option<usize> {
        (0..self.gear_slot_capacity as usize).find(|&i| self.gear[i].is_none())
    }

    /// Check if player has a specific item equipped
    pub fn has_item(&self, item_id: &[u8; 8]) -> bool {
        // Check tool
        if let Some(ref tool) = self.tool {
            if tool.item_id == *item_id {
                return true;
            }
        }
        // Check gear
        for slot in self.gear.iter().flatten() {
            if slot.item_id == *item_id {
                return true;
            }
        }
        false
    }

    /// Expand gear slots (after boss defeat)
    pub fn expand_slots(&mut self) -> Result<u8> {
        let new_capacity = match self.gear_slot_capacity {
            4 => 6,
            6 => MAX_GEAR_SLOTS,
            _ => return Err(crate::errors::InventoryError::AlreadyMaxSlots.into()),
        };
        let old_capacity = self.gear_slot_capacity;
        self.gear_slot_capacity = new_capacity;
        Ok(old_capacity)
    }
}

/// Point of Interest types that offer items
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PoiType {
    /// General supply cache - draws from all items
    SupplyCache,
    /// Tool crate - draws only Tools
    ToolCrate,
    /// Geode vault - higher rarity chance
    GeodeVault,
    /// Smuggler hatch - discounted items
    SmugglerHatch,
    /// Counter cache - only weakness tags
    CounterCache,
}

/// A generated item offer at a POI
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct ItemOffer {
    /// Which item is offered
    pub item_id: [u8; 8],
    /// Always Tier::I for fresh offers
    pub tier: Tier,
    /// Gold cost
    pub price: u16,
}
