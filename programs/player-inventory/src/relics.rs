//! Relic item definitions for asset-backed items.
//!
//! These items exist outside the base 80-item system and are backed by
//! Metaplex Core assets. They use IDs starting with "S-XX-" to avoid
//! collision with base items.

pub use crate::nft_items::{get_nft_item as get_relic_item, NFT_ITEMS as RELIC_ITEMS};
