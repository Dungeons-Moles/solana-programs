use anchor_lang::prelude::*;

/// Enemy tier determining stat scaling and Gold reward
#[derive(
    AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace, Default,
)]
#[repr(u8)]
pub enum EnemyTier {
    #[default]
    T1 = 0,
    T2 = 1,
    T3 = 2,
}

impl EnemyTier {
    /// Returns the Gold reward for defeating an enemy of this tier
    /// T1 = 2, T2 = 4, T3 = 6
    pub fn gold_reward(&self) -> u8 {
        match self {
            EnemyTier::T1 => 2,
            EnemyTier::T2 => 4,
            EnemyTier::T3 => 6,
        }
    }

    /// Create tier from u8 value
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(EnemyTier::T1),
            1 => Some(EnemyTier::T2),
            2 => Some(EnemyTier::T3),
            _ => None,
        }
    }
}

/// Stats for an enemy archetype at a specific tier
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default, InitSpace)]
pub struct EnemyStats {
    pub hp: u16,
    pub atk: u8,
    pub arm: u8,
    pub spd: u8,
    pub dig: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enemy_tier_gold_reward() {
        assert_eq!(EnemyTier::T1.gold_reward(), 2);
        assert_eq!(EnemyTier::T2.gold_reward(), 4);
        assert_eq!(EnemyTier::T3.gold_reward(), 6);
    }

    #[test]
    fn test_enemy_tier_from_u8() {
        assert_eq!(EnemyTier::from_u8(0), Some(EnemyTier::T1));
        assert_eq!(EnemyTier::from_u8(1), Some(EnemyTier::T2));
        assert_eq!(EnemyTier::from_u8(2), Some(EnemyTier::T3));
        assert_eq!(EnemyTier::from_u8(3), None);
    }
}
