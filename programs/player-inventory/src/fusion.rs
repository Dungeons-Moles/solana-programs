//! Item fusion (tier upgrade) logic for the player-inventory program.

use anchor_lang::prelude::*;

use crate::errors::InventoryError;
use crate::state::{ItemInstance, Tier};

/// Validate that two items can be fused
pub fn validate_fusion(item_a: &ItemInstance, item_b: &ItemInstance) -> Result<()> {
    // Items must have the same ID
    if item_a.item_id != item_b.item_id {
        return Err(InventoryError::FusionMismatch.into());
    }

    // Items must have the same tier
    if item_a.tier != item_b.tier {
        return Err(InventoryError::FusionMismatch.into());
    }

    // Items cannot be Tier III (already max)
    if item_a.tier == Tier::III {
        return Err(InventoryError::AlreadyMaxTier.into());
    }

    Ok(())
}

/// Execute fusion and return the new tier
pub fn execute_fusion(current_tier: Tier) -> Result<Tier> {
    current_tier
        .next()
        .ok_or(InventoryError::AlreadyMaxTier.into())
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(id: &[u8; 8], tier: Tier) -> ItemInstance {
        ItemInstance::new(*id, tier)
    }

    #[test]
    fn test_valid_fusion_tier_1() {
        let item_a = make_item(b"G-ST-01\0", Tier::I);
        let item_b = make_item(b"G-ST-01\0", Tier::I);

        assert!(validate_fusion(&item_a, &item_b).is_ok());

        let new_tier = execute_fusion(Tier::I).unwrap();
        assert_eq!(new_tier, Tier::II);
    }

    #[test]
    fn test_valid_fusion_tier_2() {
        let item_a = make_item(b"G-ST-01\0", Tier::II);
        let item_b = make_item(b"G-ST-01\0", Tier::II);

        assert!(validate_fusion(&item_a, &item_b).is_ok());

        let new_tier = execute_fusion(Tier::II).unwrap();
        assert_eq!(new_tier, Tier::III);
    }

    #[test]
    fn test_fusion_rejects_mismatched_ids() {
        let item_a = make_item(b"G-ST-01\0", Tier::I);
        let item_b = make_item(b"G-ST-02\0", Tier::I);

        let result = validate_fusion(&item_a, &item_b);
        assert!(result.is_err());
    }

    #[test]
    fn test_fusion_rejects_mismatched_tiers() {
        let item_a = make_item(b"G-ST-01\0", Tier::I);
        let item_b = make_item(b"G-ST-01\0", Tier::II);

        let result = validate_fusion(&item_a, &item_b);
        assert!(result.is_err());
    }

    #[test]
    fn test_fusion_rejects_tier_3() {
        let item_a = make_item(b"G-ST-01\0", Tier::III);
        let item_b = make_item(b"G-ST-01\0", Tier::III);

        let result = validate_fusion(&item_a, &item_b);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_fusion_tier_3_fails() {
        let result = execute_fusion(Tier::III);
        assert!(result.is_err());
    }
}
