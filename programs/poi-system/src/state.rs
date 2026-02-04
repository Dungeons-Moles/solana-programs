use anchor_lang::prelude::*;

/// Maximum number of POIs per map
pub const MAX_POIS: usize = 50;

/// Maximum number of shop offers
pub const SHOP_OFFER_COUNT: usize = 6;

/// PDA seed for MapPois account
pub const MAP_POIS_SEED: &[u8] = b"map_pois";

/// POI rarity classification
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
pub enum PoiRarity {
    /// Fixed placement (L1 Mole Den only)
    Fixed,
    /// Common POIs
    Common,
    /// Uncommon POIs
    Uncommon,
    /// Rare POIs
    Rare,
}

/// POI use type determining how often it can be used
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
pub enum UseType {
    /// Can only be used once per map
    OneTime,
    /// Can be used multiple times
    Repeatable,
    /// Can be used once per different tool
    RepeatablePerTool,
}

/// Condition for when POI can be used
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
pub enum ActiveCondition {
    /// Can be used day or night
    Anytime,
    /// Can only be used during night phases
    NightOnly,
}

/// Type of interaction the POI provides
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
pub enum InteractionType {
    /// L1: Skip to day, restore all HP
    RestFull,
    /// L5: Skip to day, heal 10 HP
    RestPartial,
    /// L2, L3, L12, L13: Pick 1 of N items
    PickItem,
    /// L4: Modify tool stats
    ToolOil,
    /// L6: Reveal tiles in radius
    MapRevealRadius,
    /// L7: Reveal nearest POI of category
    MapRevealNearest,
    /// L8: Teleport between waypoints
    FastTravel,
    /// L9: Purchase items with gold
    Shop,
    /// L10: Upgrade tool tier for gold
    Upgrade,
    /// L11: Fuse identical items
    Fuse,
    /// L14: Destroy gear for gold cost
    Scrap,
}

/// Category for Seismic Scanner (L7)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
pub enum PoiCategory {
    /// Item acquisition POIs (L2, L3, L12, L13)
    Items,
    /// Upgrade POIs (L4, L10, L11)
    Upgrades,
    /// Travel/Utility POIs (L6, L7, L8)
    Utility,
    /// Shop POI (L9)
    Shop,
}

/// Tool oil modification type
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
pub enum ToolOilModification {
    /// +1 ATK
    PlusAtk,
    /// +1 SPD
    PlusSpd,
    /// +1 DIG
    PlusDig,
    /// +1 ARM
    PlusArm,
}

/// Placement rule for spawn configuration
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
pub enum PlacementRule {
    /// Place randomly on map
    Random,
    /// Place next to starting position (L1)
    AdjacentToStart,
}

/// Single item offer at a POI or shop
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, InitSpace, Default)]
pub struct ItemOffer {
    /// Item definition ID (e.g., "T-ST-01\0")
    pub item_id: [u8; 8],
    /// Item tier (always Tier::I for fresh offers)
    pub tier: u8,
    /// Gold cost (0 for free POIs)
    pub price: u16,
    /// Whether this offer has been purchased (for shop)
    pub purchased: bool,
}

/// Offer item for cache POIs (simplified, without price/purchased).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, InitSpace, Default)]
pub struct OfferItem {
    /// Item definition ID (e.g., "G-ST-01\0")
    pub item_id: [u8; 8],
    /// Item rarity (0=Common, 1=Rare, 2=Heroic, 3=Mythic)
    pub rarity: u8,
    /// Item tier (0=Tier I, 1=Tier II, 2=Tier III). Always 0 for fresh POI drops.
    pub tier: u8,
}

/// Cache offer for pick-item POIs (L2, L3, L12, L13).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, InitSpace, Default)]
pub struct CacheOffer {
    /// Which POI this offer belongs to
    pub poi_index: u8,
    /// The 3 items offered
    pub items: [OfferItem; 3],
    /// Seed used to generate this offer (for verification)
    pub generated_at_seed: u64,
}

/// Oil offer for Tool Oil Rack (L4).
/// Stores 3 of 4 possible oils generated deterministically.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, InitSpace, Default)]
pub struct OilOffer {
    /// Which POI this offer belongs to
    pub poi_index: u8,
    /// The 3 oil flags offered (from OIL_FLAG_ATK, SPD, DIG, ARM)
    pub oils: [u8; 3],
    /// Seed used to generate this offer (for verification)
    pub generated_at_seed: u64,
}

/// Runtime POI placed on the map
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, InitSpace, Default)]
pub struct PoiInstance {
    /// References PoiDefinition ID (1-14)
    pub poi_type: u8,
    /// X coordinate on map (0-31)
    pub x: u8,
    /// Y coordinate on map (0-31)
    pub y: u8,
    /// True if one-time POI has been used
    pub used: bool,
    /// True if Rail Waypoint discovered
    pub discovered: bool,
    /// Week when spawned (for Counter Cache)
    pub week_spawned: u8,
}

/// State for active Smuggler Hatch shopping session
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, InitSpace, Default)]
pub struct ShopState {
    /// Index of Smuggler Hatch in MapPois.pois
    pub poi_index: u8,
    /// Current item offers (1 Tool + 5 Gear)
    pub offers: [ItemOffer; SHOP_OFFER_COUNT],
    /// Number of rerolls this visit
    pub reroll_count: u8,
    /// Whether shop session is active
    pub active: bool,
    /// RNG state for next reroll (for deterministic regeneration)
    pub rng_state: u64,
}

impl ShopState {
    /// Calculate the current reroll cost
    pub fn reroll_cost(&self) -> u16 {
        // 4 + (2 * reroll_count) Gold
        4 + (2 * self.reroll_count as u16)
    }
}

/// Session-scoped account holding all POI instances for a map
#[account]
#[derive(InitSpace)]
pub struct MapPois {
    /// Reference to GameSession PDA
    pub session: Pubkey,
    /// PDA bump seed
    pub bump: u8,
    /// Number of POIs
    pub count: u8,
    /// Current act (1-4)
    pub act: u8,
    /// Current week (1-3)
    pub week: u8,
    /// Seed for deterministic POI generation
    pub seed: u64,
    /// All POIs on this map
    #[max_len(50)]
    pub pois: Vec<PoiInstance>,
    /// Active shop session (if any)
    pub shop_state: ShopState,
    /// Current cache offer (for pick-item POIs)
    pub current_offer: Option<CacheOffer>,
    /// Current oil offer (for Tool Oil Rack L4)
    pub current_oil_offer: Option<OilOffer>,
}

impl MapPois {
    /// Returns the PDA seeds for this MapPois
    pub fn seeds(session: &Pubkey) -> [&[u8]; 2] {
        [MAP_POIS_SEED, session.as_ref()]
    }

    /// Get POI at a specific position
    pub fn get_poi_at_position(&self, x: u8, y: u8) -> Option<(usize, &PoiInstance)> {
        self.pois
            .iter()
            .enumerate()
            .find(|(_, poi)| poi.x == x && poi.y == y)
    }

    /// Get all discovered waypoints (L8)
    pub fn get_discovered_waypoints(&self) -> Vec<(usize, &PoiInstance)> {
        self.pois
            .iter()
            .enumerate()
            .filter(|(_, poi)| poi.poi_type == 8 && poi.discovered)
            .collect()
    }
}

/// POI info returned from view functions
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PoiInfo {
    pub id: u8,
    pub name: String,
    pub emoji: [u8; 4],
    pub rarity: PoiRarity,
    pub use_type: UseType,
    pub active_condition: ActiveCondition,
    pub interaction_type: InteractionType,
}

/// POI instance info returned from view functions
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PoiInstanceInfo {
    pub poi_type: u8,
    pub x: u8,
    pub y: u8,
    pub used: bool,
    pub discovered: bool,
    pub name: String,
}
