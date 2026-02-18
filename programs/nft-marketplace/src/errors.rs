use anchor_lang::prelude::*;

#[error_code]
pub enum MarketplaceError {
    #[msg("Unauthorized")]
    Unauthorized,

    #[msg("Invalid collection")]
    InvalidCollection,

    #[msg("Invalid price")]
    InvalidPrice,

    #[msg("NFT not owned by seller")]
    NotOwner,

    #[msg("Listing already exists")]
    ListingAlreadyExists,

    #[msg("Listing not found")]
    ListingNotFound,

    #[msg("Cannot buy your own listing")]
    CannotBuySelf,

    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,

    #[msg("Invalid Metaplex Core asset")]
    InvalidAsset,

    #[msg("Invalid mint authority")]
    InvalidMintAuthority,

    #[msg("Quest not active")]
    QuestNotActive,

    #[msg("Quest already completed")]
    QuestAlreadyCompleted,

    #[msg("Quest not completed")]
    QuestNotCompleted,

    #[msg("Quest reward already claimed")]
    QuestRewardAlreadyClaimed,

    #[msg("Invalid quest type")]
    InvalidQuestType,

    #[msg("Fee basis points exceed maximum")]
    FeeTooHigh,
}
