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

    #[msg("Invalid interaction for this POI type")]
    InvalidInteraction,
}
