//! Compatibility shim between anchor-lang 1.0.0 and ephemeral-rollups-sdk
//! (which internally depends on anchor-lang 0.32.x).
//!
//! Both anchor versions use solana AccountInfo/Pubkey with identical memory layouts.
//! We transmute at the boundary when calling ER SDK functions.

#![allow(
    clippy::missing_transmute_annotations,
    clippy::transmute_undefined_repr,
    clippy::missing_safety_doc
)]

use anchor_lang::prelude::*;

pub use ephemeral_rollups_sdk::cpi::DelegateConfig;
pub use ephemeral_rollups_sdk::pda::{
    DELEGATION_METADATA_TAG, DELEGATION_RECORD_TAG, DELEGATE_BUFFER_TAG,
};

/// Delegation program ID: DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh
pub const DELEGATION_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0xb5, 0xb7, 0x00, 0xe1, 0xf2, 0x57, 0x3a, 0xc0, 0xcc, 0x06, 0x22, 0x01, 0x34, 0x4a, 0xcf,
    0x97, 0xb8, 0x35, 0x06, 0xeb, 0x8c, 0xe5, 0x19, 0x98, 0xcc, 0x62, 0x7e, 0x18, 0x93, 0x80,
    0xa7, 0x3e,
]);

/// Magic program ID: Magic11111111111111111111111111111111111111
pub const MAGIC_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0x05, 0x45, 0xb4, 0x24, 0xb0, 0xda, 0x70, 0x95, 0xec, 0xb9, 0xd6, 0xde, 0xc3, 0x77, 0xd7,
    0x28, 0x91, 0xb6, 0xe7, 0x8e, 0x92, 0xea, 0x12, 0xd6, 0xdf, 0xbb, 0x3a, 0x40, 0x00, 0x00,
    0x00, 0x00,
]);

/// Magic context ID: MagicContext1111111111111111111111111111111
pub const MAGIC_CONTEXT_ID: Pubkey = Pubkey::new_from_array([
    0x05, 0x45, 0xb4, 0x24, 0xc4, 0xa5, 0x28, 0xbf, 0x5f, 0xb4, 0x03, 0x2f, 0x44, 0x52, 0x82,
    0x8e, 0xbb, 0x38, 0xab, 0xc1, 0xd2, 0xdc, 0x97, 0xf7, 0x3f, 0x8b, 0x94, 0x54, 0x80, 0x00,
    0x00, 0x00,
]);

/// VRF program ID: Vrf1RNUjXmQGjmQrQLvJHs9SNkvDJEsRVFPkfSQUwGz
pub const VRF_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0x07, 0x64, 0x68, 0x77, 0xc8, 0xf1, 0xfd, 0x2b, 0x9e, 0xb9, 0xf3, 0x5c, 0xea, 0xeb, 0x5d,
    0x19, 0x3f, 0xa2, 0x9e, 0xc4, 0x5c, 0x07, 0xb5, 0x65, 0xf2, 0x1b, 0xa7, 0xb1, 0x27, 0xac,
    0xf7, 0x7f,
]);

/// VRF program identity: 9irBy75QS2BN81FUgXuHcjqceJJRuc9oDkAe8TKVvvAw
pub const VRF_PROGRAM_IDENTITY: Pubkey = Pubkey::new_from_array([
    0x81, 0x95, 0xed, 0x47, 0x1b, 0xec, 0xf3, 0x74, 0xdb, 0x54, 0x22, 0xdd, 0x77, 0xe7, 0xf9,
    0xfe, 0xaa, 0xc2, 0x72, 0xa5, 0xf9, 0x5d, 0x26, 0x61, 0x92, 0x9b, 0x2f, 0x69, 0x71, 0x16,
    0x5e, 0xfc,
]);

/// VRF identity seed (same as ephemeral_vrf_sdk::consts::IDENTITY)
pub const VRF_IDENTITY_SEED: &[u8] = b"identity";

/// SlotHashes sysvar ID: SysvarS1otHashes111111111111111111111111111
pub const SLOT_HASHES_ID: Pubkey = Pubkey::new_from_array([
    0x06, 0xa7, 0xd5, 0x17, 0x19, 0x2f, 0x0a, 0xaf, 0xc6, 0xf2, 0x65, 0xe3, 0xfb, 0x77, 0xcc,
    0x7a, 0xda, 0x82, 0xc5, 0x29, 0xd0, 0xbe, 0x3b, 0x13, 0x6e, 0x2d, 0x00, 0x55, 0x20, 0x00,
    0x00, 0x00,
]);

/// Wrapper: delegate an account to the ER.
pub fn delegate_account(
    payer: &AccountInfo<'_>,
    pda: &AccountInfo<'_>,
    owner_program: &AccountInfo<'_>,
    buffer: &AccountInfo<'_>,
    delegation_record: &AccountInfo<'_>,
    delegation_metadata: &AccountInfo<'_>,
    delegation_program: &AccountInfo<'_>,
    system_program: &AccountInfo<'_>,
    pda_seeds: &[&[u8]],
    config: DelegateConfig,
) -> Result<()> {
    // SAFETY: AccountInfo layout is identical between solana-account-info 2.x and 3.x.
    // Both versions use the same repr(C)-compatible struct with identical fields and alignment.
    unsafe {
        let accounts = ephemeral_rollups_sdk::cpi::DelegateAccounts {
            payer: std::mem::transmute(payer),
            pda: std::mem::transmute(pda),
            owner_program: std::mem::transmute(owner_program),
            buffer: std::mem::transmute(buffer),
            delegation_record: std::mem::transmute(delegation_record),
            delegation_metadata: std::mem::transmute(delegation_metadata),
            delegation_program: std::mem::transmute(delegation_program),
            system_program: std::mem::transmute(system_program),
        };
        ephemeral_rollups_sdk::cpi::delegate_account(accounts, pda_seeds, config)
            .map_err(|_e| error::Error::from(ProgramError::Custom(9001)))
    }
}

/// Wrapper: undelegate an account from the ER.
pub fn undelegate_account(
    delegated_account: &AccountInfo<'_>,
    owner_program: &Pubkey,
    buffer: &AccountInfo<'_>,
    payer: &AccountInfo<'_>,
    system_program: &AccountInfo<'_>,
    account_signer_seeds: Vec<Vec<u8>>,
) -> Result<()> {
    // SAFETY: AccountInfo and Pubkey layouts are identical between solana SDK 2.x and 3.x.
    unsafe {
        ephemeral_rollups_sdk::cpi::undelegate_account(
            std::mem::transmute(delegated_account),
            std::mem::transmute(owner_program),
            std::mem::transmute(buffer),
            std::mem::transmute(payer),
            std::mem::transmute(system_program),
            account_signer_seeds,
        )
        .map_err(|_e| error::Error::from(ProgramError::Custom(9002)))
    }
}

/// Wrapper: commit and undelegate accounts via MagicIntentBundleBuilder.
pub fn commit_and_undelegate(
    session_signer: AccountInfo<'_>,
    magic_context: AccountInfo<'_>,
    magic_program: AccountInfo<'_>,
    accounts_to_undelegate: &[AccountInfo<'_>],
) -> Result<()> {
    use ephemeral_rollups_sdk::ephem::{FoldableIntentBuilder, MagicIntentBundleBuilder};

    // SAFETY: AccountInfo layout is identical between solana-account-info 2.x and 3.x.
    unsafe {
        let er_accounts: Vec<_> = accounts_to_undelegate
            .iter()
            .map(|a| std::mem::transmute_copy(a))
            .collect();

        MagicIntentBundleBuilder::new(
            std::mem::transmute(session_signer),
            std::mem::transmute(magic_context),
            std::mem::transmute(magic_program),
        )
        .commit_and_undelegate(&er_accounts)
        .build_and_invoke()
        .map_err(|_e| error::Error::from(ProgramError::Custom(9003)))
    }
}

/// Wrapper: commit accounts without undelegating.
pub fn commit_accounts(
    session_signer: AccountInfo<'_>,
    magic_context: AccountInfo<'_>,
    magic_program: AccountInfo<'_>,
    accounts_to_commit: &[AccountInfo<'_>],
) -> Result<()> {
    use ephemeral_rollups_sdk::ephem::{FoldableIntentBuilder, MagicIntentBundleBuilder};

    // SAFETY: AccountInfo layout is identical between solana-account-info 2.x and 3.x.
    unsafe {
        let er_accounts: Vec<_> = accounts_to_commit
            .iter()
            .map(|a| std::mem::transmute_copy(a))
            .collect();

        MagicIntentBundleBuilder::new(
            std::mem::transmute(session_signer),
            std::mem::transmute(magic_context),
            std::mem::transmute(magic_program),
        )
        .commit(&er_accounts)
        .build_and_invoke()
        .map_err(|_e| error::Error::from(ProgramError::Custom(9004)))
    }
}
