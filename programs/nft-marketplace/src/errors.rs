use quasar_lang::prelude::*;

#[error_code]
pub enum MarketplaceError {
    Unauthorized,
    InvalidCollection,
    InvalidPrice,
    NotOwner,
    ListingAlreadyExists,
    ListingNotFound,
    CannotBuySelf,
    ArithmeticOverflow,
    InvalidAsset,
    InvalidMintAuthority,
    QuestNotActive,
    QuestAlreadyCompleted,
    QuestNotCompleted,
    QuestRewardAlreadyClaimed,
    InvalidQuestType,
    FeeTooHigh,
    SkinCurrentlyEquipped,
    InvalidGauntletPool,
    PriceTooLow,
    InvalidAccountData,
}
