//! Error types for the player-inventory program.

use anchor_lang::prelude::*;

#[error_code]
pub enum InventoryError {
    /// No available gear slots
    #[msg("No available gear slots")]
    InventoryFull,

    /// Item ID does not exist
    #[msg("Item ID does not exist")]
    InvalidItemId,

    /// Item type does not match slot type
    #[msg("Item type does not match slot type")]
    WrongItemType,

    /// Items must have same ID and tier to fuse
    #[msg("Items must have same ID and tier to fuse")]
    FusionMismatch,

    /// Item is already at maximum tier
    #[msg("Item is already at maximum tier")]
    AlreadyMaxTier,

    /// No tool is currently equipped
    #[msg("No tool is currently equipped")]
    NoToolEquipped,

    /// This Tool Oil modification was already applied
    #[msg("This Tool Oil modification was already applied")]
    ToolOilAlreadyApplied,

    /// The specified slot is empty
    #[msg("The specified slot is empty")]
    SlotEmpty,

    /// Slot index is out of bounds
    #[msg("Slot index is out of bounds")]
    InvalidSlotIndex,

    /// Signer is not the inventory owner
    #[msg("Signer is not the inventory owner")]
    Unauthorized,

    /// Gear slots already at maximum capacity
    #[msg("Gear slots already at maximum capacity")]
    AlreadyMaxSlots,

    /// Direct player mutation path is disabled; use authorized CPI flow
    #[msg("Direct inventory mutation is disabled; use authorized CPI flow")]
    DirectMutationDisabled,
}
