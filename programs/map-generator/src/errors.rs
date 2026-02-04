use anchor_lang::prelude::*;

/// Custom error codes for the Map Generator program
#[error_code]
pub enum MapGeneratorError {
    /// Campaign level must be 0-80
    #[msg("Campaign level must be 0-80")]
    InvalidLevel,

    /// Signer is not the config admin
    #[msg("Signer is not the config admin")]
    Unauthorized,

    /// Map config is already initialized
    #[msg("Map config is already initialized")]
    ConfigAlreadyInitialized,

    /// Map hash does not match expected value
    #[msg("Map hash does not match expected value")]
    InvalidMapHash,

    /// Batch update exceeds maximum allowed
    #[msg("Batch update exceeds maximum allowed")]
    TooManyUpdates,

    /// Map generation failed (could not find valid spawn point)
    #[msg("Map generation failed")]
    MapGenerationFailed,

    /// Map already exists for this session
    #[msg("Map already exists for this session")]
    MapAlreadyExists,

    /// Session account is not owned by session manager
    #[msg("Invalid session owner")]
    InvalidSessionOwner,

    /// POI index is out of bounds
    #[msg("POI index out of bounds")]
    InvalidPoiIndex,

    /// Tile position is out of bounds
    #[msg("Tile position out of bounds")]
    TileOutOfBounds,

    /// Invalid session account for close authorization
    #[msg("Invalid session account")]
    InvalidSession,
}
