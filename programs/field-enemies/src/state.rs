use anchor_lang::prelude::*;

/// Enemy tier determining stat scaling and Gold reward
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
#[repr(u8)]
pub enum EnemyTier {
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

impl Default for EnemyTier {
    fn default() -> Self {
        EnemyTier::T1
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

/// A spawned enemy instance on the map (5 bytes)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default, InitSpace)]
pub struct EnemyInstance {
    /// References EnemyArchetype ID (0-11)
    pub archetype_id: u8,
    /// Tier: 0=T1, 1=T2, 2=T3
    pub tier: u8,
    /// Map X coordinate
    pub x: u8,
    /// Map Y coordinate
    pub y: u8,
    /// True if already defeated
    pub defeated: bool,
}

impl EnemyInstance {
    /// Get the tier enum for this instance
    pub fn get_tier(&self) -> EnemyTier {
        EnemyTier::from_u8(self.tier).unwrap_or_default()
    }
}

/// Maximum number of enemies per map (Act 4 max)
pub const MAX_ENEMIES: usize = 48;

/// On-chain account storing all enemy instances for a map
/// PDA Seeds: ["map_enemies", session.as_ref()]
#[account]
#[derive(InitSpace)]
pub struct MapEnemies {
    /// Parent session PDA
    pub session: Pubkey,

    /// Enemy instances (max 48)
    #[max_len(48)]
    pub enemies: Vec<EnemyInstance>,

    /// Actual count of enemies
    pub count: u8,

    /// PDA bump seed
    pub bump: u8,
}

impl MapEnemies {
    /// PDA seed prefix
    pub const SEED_PREFIX: &'static [u8] = b"map_enemies";

    /// Find enemy at given position
    pub fn get_enemy_at_position(&self, x: u8, y: u8) -> Option<&EnemyInstance> {
        self.enemies
            .iter()
            .find(|e| e.x == x && e.y == y && !e.defeated)
    }

    /// Find enemy at given position (mutable)
    pub fn get_enemy_at_position_mut(&mut self, x: u8, y: u8) -> Option<&mut EnemyInstance> {
        self.enemies
            .iter_mut()
            .find(|e| e.x == x && e.y == y && !e.defeated)
    }
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
