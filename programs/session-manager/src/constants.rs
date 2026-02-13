/// Map width in tiles (fixed for all sessions)
pub const MAP_WIDTH: u8 = 50;

/// Map height in tiles (fixed for all sessions)
pub const MAP_HEIGHT: u8 = 50;

/// Campaign levels per act
pub const LEVELS_PER_ACT: u8 = 10;

/// Fixed campaign level used for Duels difficulty tuning.
/// Level 20 gives a mid-road encounter profile.
pub const DUEL_CAMPAIGN_LEVEL: u8 = 20;

/// Fixed campaign level used for Gauntlet difficulty tuning.
/// Gauntlet shares the same difficulty profile as Duels.
pub const GAUNTLET_CAMPAIGN_LEVEL: u8 = 20;
