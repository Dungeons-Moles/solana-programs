use anchor_lang::prelude::*;

#[error_code]
pub enum PoiSystemError {
    #[msg("Act must be 1-4")]
    InvalidAct,

    #[msg("POI type must be 1-14")]
    InvalidPoiType,

    #[msg("No POI at specified position")]
    PoiNotFound,

    #[msg("One-time POI already used")]
    PoiAlreadyUsed,

    #[msg("This POI can only be used at night")]
    NightOnlyPoi,

    #[msg("Player has no tool equipped")]
    NoToolEquipped,

    #[msg("Tool oil modification already applied")]
    OilAlreadyApplied,

    #[msg("Not enough gold")]
    InsufficientGold,

    #[msg("No space in inventory")]
    InventoryFull,

    #[msg("Items must be identical for fusion")]
    ItemsNotIdentical,

    #[msg("Item is already maximum tier")]
    AlreadyMaxTier,

    #[msg("No active shop session")]
    ShopNotActive,

    #[msg("Offer already purchased")]
    OfferAlreadyPurchased,

    #[msg("Waypoint destination not discovered")]
    DestinationNotDiscovered,

    #[msg("No other discovered waypoints")]
    NoDestinationsAvailable,

    #[msg("Offer index out of bounds")]
    InvalidOfferIndex,

    #[msg("Gear slot index out of bounds or empty")]
    InvalidGearSlot,

    #[msg("POI index out of bounds")]
    InvalidPoiIndex,

    #[msg("Unauthorized: only session owner can interact")]
    Unauthorized,

    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,

    #[msg("POIs already initialized for this session")]
    AlreadyInitialized,

    #[msg("Shop already active")]
    ShopAlreadyActive,

    #[msg("Shop reroll limit reached for this visit")]
    ShopRerollLimitReached,

    #[msg("Invalid interaction for this POI type")]
    InvalidInteraction,

    #[msg("No POI interaction is currently active")]
    NoActiveInteraction,

    #[msg("An offer is already active for this interaction; resolve it before regenerating")]
    OfferAlreadyGenerated,

    #[msg("No items available for the selected tag and rarity")]
    NoItemsAvailable,

    #[msg("Invalid offer context")]
    InvalidOfferContext,

    #[msg("Player is not on the POI tile")]
    PlayerNotOnPoiTile,

    #[msg("Session account is not owned by session-manager program")]
    InvalidSessionOwner,

    #[msg("Invalid generated map account")]
    InvalidGeneratedMap,

    #[msg("Invalid game session account")]
    InvalidSession,

    #[msg("Week must be 1-3")]
    InvalidWeek,

    #[msg("Failed to fetch boss weaknesses")]
    InvalidBossWeek,

    #[msg("Selected oil is not in the generated offer")]
    InvalidOilSelection,

    #[msg("Item is not currently equipped in inventory")]
    ItemNotInInventory,

    #[msg("Invalid vision radius for current phase/state")]
    InvalidVisionRadius,

    #[msg("VRF not fulfilled")]
    VrfNotFulfilled,

    #[msg("VRF not requested")]
    VrfNotRequested,

    #[msg("VRF randomness required for PvP offer generation")]
    VrfRequired,
}
