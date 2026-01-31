//! Constants for the player-inventory program.

/// Initial number of gear slots for new players
pub const INITIAL_GEAR_SLOTS: u8 = 4;

/// Maximum number of gear slots (after all expansions)
pub const MAX_GEAR_SLOTS: u8 = 8;

/// Gear slots after defeating Week 1 boss
pub const WEEK1_GEAR_SLOTS: u8 = 6;

/// Tool Oil flag for +1 ATK
pub const TOOL_OIL_ATK: u8 = 0x01;

/// Tool Oil flag for +1 SPD
pub const TOOL_OIL_SPD: u8 = 0x02;

/// Tool Oil flag for +1 DIG
pub const TOOL_OIL_DIG: u8 = 0x04;

/// Tool Oil flag for +1 ARM
pub const TOOL_OIL_ARM: u8 = 0x08;
