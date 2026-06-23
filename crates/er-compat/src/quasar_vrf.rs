//! Quasar-compatible MagicBlock VRF adapter.
//!
//! This mirrors `ephemeral-rollups-pinocchio`'s scoped VRF request contract
//! without depending on Pinocchio account-view types at runtime.

use quasar_lang::{
    cpi::{CpiDynamic, CpiSignerSeeds, Seed},
    prelude::{address, AccountView, Address, ProgramError},
};

use pinocchio::AccountView as PinocchioAccountView;

pub type ProgramResult = Result<(), ProgramError>;

pub use ephemeral_rollups_pinocchio::{
    consts::{
        DELEGATION_PROGRAM_ID, EXTERNAL_UNDELEGATE_DISCRIMINATOR, MAGIC_CONTEXT_ID,
        MAGIC_PROGRAM_ID,
    },
    pda::{
        delegate_buffer_pda_from_delegated_account_and_owner_program,
        delegation_metadata_pda_from_delegated_account,
        delegation_record_pda_from_delegated_account,
    },
};

/// VRF program id: Vrf1RNUjXmQGjmQrQLvJHs9SNkvDJEsRVFPkfSQUwGz.
pub const VRF_PROGRAM_ID: Address = address!("Vrf1RNUjXmQGjmQrQLvJHs9SNkvDJEsRVFPkfSQUwGz");

/// Base-layer default queue. The adapter never accepts this for game VRF.
pub const DEFAULT_QUEUE: Address = address!("Cuj97ggrhhidhbu39TijNVqE74xvKJ69gDervRUXAxGh");

/// ER oracle queue required by D&M VRF rules.
pub const ER_ORACLE_QUEUE: Address = address!("5hBR571xnXppuCPveTrctfTU7tJLSN94nq7kv7FRK5Tc");

/// MagicBlock's generated local ER queue used by the upstream Pinocchio example.
///
/// Do not use this for production D&M VRF. It exists only so the throwaway
/// localnet probe can run against the queue serviced by `vrf-oracle` locally.
pub const LOCAL_SERVICEABLE_ER_QUEUE: Address =
    address!("Sc9MJUngNbQXSXGP3F67KvKwVnhaYn6kcioxXNVowYT");

/// Deprecated global VRF identity; scoped callbacks must not validate this.
pub const DEPRECATED_GLOBAL_VRF_PROGRAM_IDENTITY: Address =
    address!("9irBy75QS2BN81FUgXuHcjqceJJRuc9oDkAe8TKVvvAw");

/// Solana system program id.
pub const SYSTEM_PROGRAM_ID: Address = address!("11111111111111111111111111111111");

/// SlotHashes sysvar id.
pub const SLOT_HASHES_ID: Address = address!("SysvarS1otHashes111111111111111111111111111");

/// Program identity seed used by the callback program and scoped VRF identity.
pub const IDENTITY_SEED: &[u8] = b"identity";

/// MagicBlock scoped randomness discriminator.
pub const REQUEST_SCOPED_RANDOMNESS_DISCRIMINATOR: u64 = 10;

/// MagicBlock high-priority scoped randomness discriminator.
pub const REQUEST_HIGH_PRIORITY_SCOPED_RANDOMNESS_DISCRIMINATOR: u64 = 11;

/// Number of accounts in the VRF request instruction.
pub const REQUEST_RANDOMNESS_ACCOUNTS: usize = 5;

/// Discriminator prefix length in MagicBlock VRF request data.
pub const DISCRIMINATOR_PREFIX_SIZE: usize = 8;

/// Serialized callback account meta size: 32 pubkey + signer byte + writable byte.
pub const SERIALIZABLE_ACCOUNT_META_SIZE: usize = 34;

pub const ERROR_INVALID_ER_ORACLE_QUEUE: u32 = 9_201;
pub const ERROR_INVALID_VRF_PROGRAM: u32 = 9_202;
pub const ERROR_INVALID_PROGRAM_IDENTITY: u32 = 9_203;
pub const ERROR_INVALID_SYSTEM_PROGRAM: u32 = 9_204;
pub const ERROR_INVALID_SLOT_HASHES: u32 = 9_205;
pub const ERROR_INVALID_SCOPED_VRF_IDENTITY: u32 = 9_206;
pub const ERROR_INVALID_DELEGATION_PROGRAM: u32 = 9_207;
pub const ERROR_INVALID_MAGIC_PROGRAM: u32 = 9_208;
pub const ERROR_INVALID_MAGIC_CONTEXT: u32 = 9_209;

const INTENT_BUNDLE_SIZE: usize = 512;

#[inline(always)]
fn as_pinocchio_account_view(view: &AccountView) -> &PinocchioAccountView {
    // SAFETY: This spike bridges Quasar's solana-account-view 2.x wrapper into
    // MagicBlock's Pinocchio SDK, which still depends on solana-account-view
    // 1.x. Both are repr(C) single-pointer views over the same runtime account.
    unsafe { &*(view as *const AccountView as *const PinocchioAccountView) }
}

#[inline(always)]
fn clone_pinocchio_account_view(view: &AccountView) -> PinocchioAccountView {
    as_pinocchio_account_view(view).clone()
}

/// Callback account metadata serialized into MagicBlock VRF request data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallbackAccountMeta {
    pub address: Address,
    pub is_signer: bool,
    pub is_writable: bool,
}

/// Fixed account metas for a scoped VRF request instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestAccountMeta {
    pub address: Address,
    pub is_signer: bool,
    pub is_writable: bool,
}

/// Borsh-compatible MagicBlock scoped `RequestRandomness` payload.
pub struct RequestRandomness<'a> {
    pub high_priority: bool,
    pub caller_seed: [u8; 32],
    pub callback_program_id: &'a Address,
    pub callback_discriminator: &'a [u8],
    pub callback_accounts_metas: &'a [CallbackAccountMeta],
    pub callback_args: &'a [u8],
}

impl RequestRandomness<'_> {
    pub const fn serialized_size_for(
        callback_discriminator_len: usize,
        callback_accounts_metas_len: usize,
        callback_args_len: usize,
    ) -> usize {
        DISCRIMINATOR_PREFIX_SIZE
            + 32
            + 32
            + 4
            + callback_discriminator_len
            + 4
            + callback_accounts_metas_len * SERIALIZABLE_ACCOUNT_META_SIZE
            + 4
            + callback_args_len
    }

    pub fn serialized_size(&self) -> usize {
        Self::serialized_size_for(
            self.callback_discriminator.len(),
            self.callback_accounts_metas.len(),
            self.callback_args.len(),
        )
    }

    #[inline(always)]
    pub fn discriminator(&self) -> u64 {
        if self.high_priority {
            REQUEST_HIGH_PRIORITY_SCOPED_RANDOMNESS_DISCRIMINATOR
        } else {
            REQUEST_SCOPED_RANDOMNESS_DISCRIMINATOR
        }
    }

    pub fn serialize_into(&self, data: &mut [u8]) -> Result<usize, ProgramError> {
        let required = self.serialized_size();
        if data.len() < required {
            return Err(ProgramError::InvalidInstructionData);
        }

        let mut offset = 0usize;
        write_bytes(
            data,
            &mut offset,
            self.discriminator().to_le_bytes().as_ref(),
        )?;
        write_bytes(data, &mut offset, &self.caller_seed)?;
        write_bytes(data, &mut offset, self.callback_program_id.as_ref())?;

        write_len(data, &mut offset, self.callback_discriminator.len())?;
        write_bytes(data, &mut offset, self.callback_discriminator)?;

        write_len(data, &mut offset, self.callback_accounts_metas.len())?;
        for meta in self.callback_accounts_metas {
            write_bytes(data, &mut offset, meta.address.as_ref())?;
            write_bytes(data, &mut offset, &[meta.is_signer as u8])?;
            write_bytes(data, &mut offset, &[meta.is_writable as u8])?;
        }

        write_len(data, &mut offset, self.callback_args.len())?;
        write_bytes(data, &mut offset, self.callback_args)?;

        Ok(offset)
    }
}

#[inline(always)]
fn write_len(data: &mut [u8], offset: &mut usize, len: usize) -> Result<(), ProgramError> {
    if len > u32::MAX as usize {
        return Err(ProgramError::InvalidInstructionData);
    }
    write_bytes(data, offset, &(len as u32).to_le_bytes())
}

#[inline(always)]
fn write_bytes(data: &mut [u8], offset: &mut usize, bytes: &[u8]) -> Result<(), ProgramError> {
    let end = offset
        .checked_add(bytes.len())
        .ok_or(ProgramError::InvalidInstructionData)?;
    if end > data.len() {
        return Err(ProgramError::InvalidInstructionData);
    }
    data[*offset..end].copy_from_slice(bytes);
    *offset = end;
    Ok(())
}

/// Derive the callback program's `[b"identity"]` PDA that signs VRF requests.
pub fn program_identity_pda(program_id: &Address) -> Result<(Address, u8), ProgramError> {
    Ok(Address::find_program_address(&[IDENTITY_SEED], program_id))
}

/// Derive the scoped VRF callback signer PDA for `callback_program_id`.
pub fn scoped_vrf_identity(callback_program_id: &Address) -> Result<(Address, u8), ProgramError> {
    Ok(Address::find_program_address(
        &[IDENTITY_SEED, callback_program_id.as_ref()],
        &VRF_PROGRAM_ID,
    ))
}

/// Expected account order for MagicBlock scoped VRF requests.
pub fn request_account_metas(
    payer: &Address,
    callback_program_id: &Address,
    oracle_queue: &Address,
) -> Result<[RequestAccountMeta; REQUEST_RANDOMNESS_ACCOUNTS], ProgramError> {
    let (program_identity, _) = program_identity_pda(callback_program_id)?;
    Ok([
        RequestAccountMeta {
            address: *payer,
            is_signer: true,
            is_writable: true,
        },
        RequestAccountMeta {
            address: program_identity,
            is_signer: true,
            is_writable: false,
        },
        RequestAccountMeta {
            address: *oracle_queue,
            is_signer: false,
            is_writable: true,
        },
        RequestAccountMeta {
            address: SYSTEM_PROGRAM_ID,
            is_signer: false,
            is_writable: false,
        },
        RequestAccountMeta {
            address: SLOT_HASHES_ID,
            is_signer: false,
            is_writable: false,
        },
    ])
}

/// Account views needed to CPI into MagicBlock VRF from a Quasar program.
pub struct RequestRandomnessAccounts<'a> {
    pub payer: &'a AccountView,
    pub program_identity: &'a AccountView,
    pub oracle_queue: &'a AccountView,
    pub system_program: &'a AccountView,
    pub slot_hashes: &'a AccountView,
    pub vrf_program: &'a AccountView,
}

pub fn validate_request_accounts(
    accounts: &RequestRandomnessAccounts<'_>,
    callback_program_id: &Address,
) -> ProgramResult {
    validate_request_accounts_for_queue(accounts, callback_program_id, &ER_ORACLE_QUEUE)
}

pub fn validate_request_accounts_for_queue(
    accounts: &RequestRandomnessAccounts<'_>,
    callback_program_id: &Address,
    expected_oracle_queue: &Address,
) -> ProgramResult {
    if *accounts.vrf_program.address() != VRF_PROGRAM_ID {
        return Err(ProgramError::Custom(ERROR_INVALID_VRF_PROGRAM));
    }
    if accounts.oracle_queue.address() != expected_oracle_queue {
        return Err(ProgramError::Custom(ERROR_INVALID_ER_ORACLE_QUEUE));
    }
    if *accounts.system_program.address() != SYSTEM_PROGRAM_ID {
        return Err(ProgramError::Custom(ERROR_INVALID_SYSTEM_PROGRAM));
    }
    if *accounts.slot_hashes.address() != SLOT_HASHES_ID {
        return Err(ProgramError::Custom(ERROR_INVALID_SLOT_HASHES));
    }
    if !accounts.payer.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if !accounts.payer.is_writable() || !accounts.oracle_queue.is_writable() {
        return Err(ProgramError::InvalidArgument);
    }

    let (expected_identity, _) = program_identity_pda(callback_program_id)?;
    if *accounts.program_identity.address() != expected_identity {
        return Err(ProgramError::Custom(ERROR_INVALID_PROGRAM_IDENTITY));
    }

    Ok(())
}

/// Request scoped VRF on the ER queue using explicit signer seeds.
pub fn request_scoped_ephemeral_vrf<const MAX_DATA: usize, S: CpiSignerSeeds + ?Sized>(
    accounts: &RequestRandomnessAccounts<'_>,
    request: &RequestRandomness<'_>,
    signer_seeds: &S,
) -> ProgramResult {
    request_scoped_ephemeral_vrf_for_queue::<MAX_DATA, S>(
        accounts,
        request,
        signer_seeds,
        &ER_ORACLE_QUEUE,
    )
}

/// Request scoped VRF against an explicit queue.
///
/// Production D&M code should use `request_scoped_ephemeral_vrf`, which pins the
/// ER queue. This explicit variant is for local harnesses where MagicBlock's
/// packaged oracle services a generated local queue.
pub fn request_scoped_ephemeral_vrf_for_queue<const MAX_DATA: usize, S: CpiSignerSeeds + ?Sized>(
    accounts: &RequestRandomnessAccounts<'_>,
    request: &RequestRandomness<'_>,
    signer_seeds: &S,
    expected_oracle_queue: &Address,
) -> ProgramResult {
    validate_request_accounts_for_queue(
        accounts,
        request.callback_program_id,
        expected_oracle_queue,
    )?;

    let written = request.serialized_size();
    if written > MAX_DATA {
        return Err(ProgramError::InvalidInstructionData);
    }

    let mut cpi = CpiDynamic::<REQUEST_RANDOMNESS_ACCOUNTS, MAX_DATA>::new(&VRF_PROGRAM_ID);
    cpi.push_account(accounts.payer, true, true)?;
    cpi.push_account(accounts.program_identity, true, false)?;
    cpi.push_account(accounts.oracle_queue, false, true)?;
    cpi.push_account(accounts.system_program, false, false)?;
    cpi.push_account(accounts.slot_hashes, false, false)?;

    let data = unsafe { &mut *cpi.data_mut() };
    let data_len = request.serialize_into(&mut data[..])?;
    unsafe { cpi.set_data_len(data_len)? };
    cpi.invoke_signed(signer_seeds)
}

/// Convenience wrapper for the callback program's `[b"identity"]` PDA bump.
pub fn request_scoped_ephemeral_vrf_with_identity_bump<const MAX_DATA: usize>(
    accounts: &RequestRandomnessAccounts<'_>,
    request: &RequestRandomness<'_>,
    identity_bump: u8,
) -> ProgramResult {
    request_scoped_ephemeral_vrf_with_identity_bump_for_queue::<MAX_DATA>(
        accounts,
        request,
        identity_bump,
        &ER_ORACLE_QUEUE,
    )
}

pub fn request_scoped_ephemeral_vrf_with_identity_bump_for_queue<const MAX_DATA: usize>(
    accounts: &RequestRandomnessAccounts<'_>,
    request: &RequestRandomness<'_>,
    identity_bump: u8,
    expected_oracle_queue: &Address,
) -> ProgramResult {
    let bump = [identity_bump];
    let seeds = [Seed::from(IDENTITY_SEED), Seed::from(bump.as_ref())];
    request_scoped_ephemeral_vrf_for_queue::<MAX_DATA, _>(
        accounts,
        request,
        &seeds,
        expected_oracle_queue,
    )
}

/// Verify the signer MagicBlock passes to a scoped VRF callback.
pub fn verify_scoped_vrf_identity(
    callback_program_id: &Address,
    scoped_identity: &AccountView,
) -> ProgramResult {
    if !scoped_identity.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    let (expected, _) = scoped_vrf_identity(callback_program_id)?;
    if *scoped_identity.address() != expected {
        return Err(ProgramError::Custom(ERROR_INVALID_SCOPED_VRF_IDENTITY));
    }
    Ok(())
}

/// Decode the 32-byte VRF randomness from raw callback data.
///
/// Use this for raw callback handlers where the instruction discriminator is
/// still present. In normal Quasar handlers, the framework has already selected
/// the instruction and the randomness can be accepted as a `[u8; 32]` argument.
pub fn decode_callback_randomness(
    data: &[u8],
    callback_discriminator_len: usize,
) -> Result<[u8; 32], ProgramError> {
    let start = callback_discriminator_len;
    let end = start
        .checked_add(32)
        .ok_or(ProgramError::InvalidInstructionData)?;
    if data.len() < end {
        return Err(ProgramError::InvalidInstructionData);
    }
    let mut randomness = [0u8; 32];
    randomness.copy_from_slice(&data[start..end]);
    Ok(randomness)
}

/// Account views needed to delegate a Quasar-owned PDA to MagicBlock ER.
pub struct DelegateAccountAccounts<'a> {
    pub payer: &'a AccountView,
    pub delegated_account: &'a AccountView,
    pub owner_program: &'a AccountView,
    pub buffer: &'a AccountView,
    pub delegation_record: &'a AccountView,
    pub delegation_metadata: &'a AccountView,
    pub system_program: &'a AccountView,
}

/// Delegate a Quasar PDA using MagicBlock's no_std Pinocchio delegation helper.
pub fn delegate_account_to_ephemeral(
    accounts: &DelegateAccountAccounts<'_>,
    seeds: &[&[u8]],
    bump: u8,
    validator: Option<Address>,
) -> ProgramResult {
    ephemeral_rollups_pinocchio::instruction::DelegateAccountCpiBuilder::new(
        as_pinocchio_account_view(accounts.payer),
        as_pinocchio_account_view(accounts.delegated_account),
        as_pinocchio_account_view(accounts.owner_program),
        as_pinocchio_account_view(accounts.buffer),
        as_pinocchio_account_view(accounts.delegation_record),
        as_pinocchio_account_view(accounts.delegation_metadata),
        as_pinocchio_account_view(accounts.system_program),
    )
    .config(ephemeral_rollups_pinocchio::types::DelegateConfig {
        commit_frequency_ms: 0,
        validator,
    })
    .seeds(seeds)
    .bump(bump)
    .invoke()
}

/// Account views needed to schedule a commit+undelegate intent on ER.
pub struct CommitAndUndelegateAccounts<'a> {
    pub payer: &'a AccountView,
    pub magic_context: &'a AccountView,
    pub magic_program: &'a AccountView,
    pub delegated_account: &'a AccountView,
}

/// Schedule commit+undelegate for one delegated account.
pub fn commit_and_undelegate_account(accounts: &CommitAndUndelegateAccounts<'_>) -> ProgramResult {
    if *accounts.magic_program.address() != MAGIC_PROGRAM_ID {
        return Err(ProgramError::Custom(ERROR_INVALID_MAGIC_PROGRAM));
    }
    if *accounts.magic_context.address() != MAGIC_CONTEXT_ID {
        return Err(ProgramError::Custom(ERROR_INVALID_MAGIC_CONTEXT));
    }

    let mut data = [0u8; INTENT_BUNDLE_SIZE];
    let delegated_accounts = [clone_pinocchio_account_view(accounts.delegated_account)];
    ephemeral_rollups_pinocchio::intent_bundle::MagicIntentBundleBuilder::new(
        clone_pinocchio_account_view(accounts.payer),
        clone_pinocchio_account_view(accounts.magic_context),
        clone_pinocchio_account_view(accounts.magic_program),
    )
    .commit_and_undelegate(&delegated_accounts)
    .build_and_invoke(&mut data)
}

/// Account views passed by MagicBlock's external undelegate callback.
pub struct ExternalUndelegateAccounts<'a> {
    pub delegated_account: &'a AccountView,
    pub buffer: &'a AccountView,
    pub payer: &'a AccountView,
}

/// Recreate a delegated PDA on base when MagicBlock invokes the external callback.
pub fn process_external_undelegate(
    accounts: &ExternalUndelegateAccounts<'_>,
    owner_program: &Address,
    callback_args: &[u8],
) -> ProgramResult {
    ephemeral_rollups_pinocchio::instruction::undelegate(
        as_pinocchio_account_view(accounts.delegated_account),
        owner_program,
        as_pinocchio_account_view(accounts.buffer),
        as_pinocchio_account_view(accounts.payer),
        callback_args,
    )
}

#[cfg(test)]
mod tests {
    extern crate std;

    use std::{vec, vec::Vec};

    use ephemeral_rollups_pinocchio::vrf as pinocchio_vrf;
    use pinocchio::{
        instruction::InstructionAccount as PinocchioInstructionAccount, Address as PinocchioAddress,
    };
    use solana_program::pubkey::Pubkey;

    use super::*;

    type MetaTuple = (Address, bool, bool);

    fn qaddr(byte: u8) -> Address {
        Address::new_from_array([byte; 32])
    }

    fn paddr(address: &Address) -> PinocchioAddress {
        PinocchioAddress::new_from_array(address.to_bytes())
    }

    fn serialize_ours(
        caller_seed: [u8; 32],
        callback_program_id: Address,
        discriminator: &[u8],
        metas: &[MetaTuple],
        callback_args: &[u8],
        high_priority: bool,
    ) -> Vec<u8> {
        let callback_accounts_metas: Vec<CallbackAccountMeta> = metas
            .iter()
            .map(|(address, is_signer, is_writable)| CallbackAccountMeta {
                address: *address,
                is_signer: *is_signer,
                is_writable: *is_writable,
            })
            .collect();
        let request = RequestRandomness {
            high_priority,
            caller_seed,
            callback_program_id: &callback_program_id,
            callback_discriminator: discriminator,
            callback_accounts_metas: &callback_accounts_metas,
            callback_args,
        };
        let mut data = vec![0u8; request.serialized_size()];
        let written = request.serialize_into(&mut data).unwrap();
        data.truncate(written);
        data
    }

    fn serialize_pinocchio(
        caller_seed: [u8; 32],
        callback_program_id: Address,
        discriminator: &[u8],
        metas: &[MetaTuple],
        callback_args: &[u8],
        high_priority: bool,
    ) -> Vec<u8> {
        let callback_program_id = paddr(&callback_program_id);
        let meta_addresses: Vec<PinocchioAddress> =
            metas.iter().map(|(address, _, _)| paddr(address)).collect();
        let pinocchio_metas: Vec<PinocchioInstructionAccount> = metas
            .iter()
            .enumerate()
            .map(|(idx, (_, is_signer, is_writable))| {
                PinocchioInstructionAccount::new(&meta_addresses[idx], *is_writable, *is_signer)
            })
            .collect();
        let request = pinocchio_vrf::RequestRandomness {
            high_priority,
            caller_seed,
            callback_program_id: &callback_program_id,
            callback_discriminator: discriminator,
            callback_accounts_metas: &pinocchio_metas,
            callback_args,
        };
        let mut data = vec![0u8; request.serialized_size()];
        let discriminator = if high_priority {
            pinocchio_vrf::REQUEST_HIGH_PRIORITY_SCOPED_RANDOMNESS_DISCRIMINATOR
        } else {
            pinocchio_vrf::REQUEST_SCOPED_RANDOMNESS_DISCRIMINATOR
        };
        let written = request.serialize_into(&mut data, discriminator).unwrap();
        data.truncate(written);
        data
    }

    #[test]
    fn constants_match_magicblock_pinocchio_vrf() {
        assert_eq!(
            VRF_PROGRAM_ID.as_ref(),
            pinocchio_vrf::VRF_PROGRAM_ID.as_ref()
        );
        assert_eq!(
            DEFAULT_QUEUE.as_ref(),
            pinocchio_vrf::DEFAULT_QUEUE.as_ref()
        );
        assert_eq!(
            ER_ORACLE_QUEUE.as_ref(),
            pinocchio_vrf::DEFAULT_EPHEMERAL_QUEUE.as_ref()
        );
        assert_eq!(
            DEPRECATED_GLOBAL_VRF_PROGRAM_IDENTITY.as_ref(),
            pinocchio_vrf::VRF_PROGRAM_IDENTITY.as_ref()
        );
        assert_eq!(IDENTITY_SEED, pinocchio_vrf::IDENTITY_SEED);
        assert_eq!(
            REQUEST_SCOPED_RANDOMNESS_DISCRIMINATOR,
            pinocchio_vrf::REQUEST_SCOPED_RANDOMNESS_DISCRIMINATOR
        );
        assert_eq!(
            REQUEST_HIGH_PRIORITY_SCOPED_RANDOMNESS_DISCRIMINATOR,
            pinocchio_vrf::REQUEST_HIGH_PRIORITY_SCOPED_RANDOMNESS_DISCRIMINATOR
        );
        assert_ne!(ER_ORACLE_QUEUE, DEFAULT_QUEUE);
    }

    #[test]
    fn request_bytes_match_pinocchio_regular_scoped() {
        let metas = [
            (qaddr(1), true, false),
            (qaddr(2), false, true),
            (qaddr(3), true, true),
        ];
        let ours = serialize_ours(
            [7u8; 32],
            qaddr(9),
            &[10, 20, 30, 40, 50, 60, 70, 80],
            &metas,
            &[100, 101, 102],
            false,
        );
        let upstream = serialize_pinocchio(
            [7u8; 32],
            qaddr(9),
            &[10, 20, 30, 40, 50, 60, 70, 80],
            &metas,
            &[100, 101, 102],
            false,
        );
        assert_eq!(ours, upstream);
        assert_eq!(
            &ours[..8],
            REQUEST_SCOPED_RANDOMNESS_DISCRIMINATOR
                .to_le_bytes()
                .as_ref()
        );
    }

    #[test]
    fn request_bytes_match_pinocchio_high_priority_scoped() {
        let metas = [(qaddr(14), true, true), (qaddr(15), false, false)];
        let ours = serialize_ours(
            [12u8; 32],
            qaddr(13),
            &[1, 1, 2, 3, 5, 8, 13, 21],
            &metas,
            &[34, 55],
            true,
        );
        let upstream = serialize_pinocchio(
            [12u8; 32],
            qaddr(13),
            &[1, 1, 2, 3, 5, 8, 13, 21],
            &metas,
            &[34, 55],
            true,
        );
        assert_eq!(ours, upstream);
        assert_eq!(
            &ours[..8],
            REQUEST_HIGH_PRIORITY_SCOPED_RANDOMNESS_DISCRIMINATOR
                .to_le_bytes()
                .as_ref()
        );
    }

    #[test]
    fn request_bytes_match_empty_metas_and_args() {
        let ours = serialize_ours([0u8; 32], qaddr(0xab), &[42], &[], &[], false);
        let upstream = serialize_pinocchio([0u8; 32], qaddr(0xab), &[42], &[], &[], false);
        assert_eq!(ours, upstream);
    }

    #[test]
    fn pda_derivations_match_pinocchio() {
        for salt in [0u8, 1, 42, 200] {
            let callback_program_id = qaddr(salt);

            let (ours_identity, ours_identity_bump) =
                program_identity_pda(&callback_program_id).unwrap();
            let (pinocchio_identity, pinocchio_identity_bump) =
                pinocchio_vrf::program_identity_pda(&paddr(&callback_program_id));
            assert_eq!(ours_identity.as_ref(), pinocchio_identity.as_ref());
            assert_eq!(ours_identity_bump, pinocchio_identity_bump);

            let (ours_scoped, ours_scoped_bump) =
                scoped_vrf_identity(&callback_program_id).unwrap();
            let (pinocchio_scoped, pinocchio_scoped_bump) =
                pinocchio_vrf::scoped_vrf_identity(&paddr(&callback_program_id));
            assert_eq!(ours_scoped.as_ref(), pinocchio_scoped.as_ref());
            assert_eq!(ours_scoped_bump, pinocchio_scoped_bump);
        }
    }

    #[test]
    fn request_account_order_matches_magicblock_sdk_contract() {
        let payer = qaddr(5);
        let callback_program_id = qaddr(9);
        let metas = request_account_metas(&payer, &callback_program_id, &ER_ORACLE_QUEUE).unwrap();
        let (program_identity, _) = program_identity_pda(&callback_program_id).unwrap();

        assert_eq!(
            metas,
            [
                RequestAccountMeta {
                    address: payer,
                    is_signer: true,
                    is_writable: true,
                },
                RequestAccountMeta {
                    address: program_identity,
                    is_signer: true,
                    is_writable: false,
                },
                RequestAccountMeta {
                    address: ER_ORACLE_QUEUE,
                    is_signer: false,
                    is_writable: true,
                },
                RequestAccountMeta {
                    address: SYSTEM_PROGRAM_ID,
                    is_signer: false,
                    is_writable: false,
                },
                RequestAccountMeta {
                    address: SLOT_HASHES_ID,
                    is_signer: false,
                    is_writable: false,
                },
            ]
        );
    }

    #[test]
    fn scoped_identity_is_not_legacy_global_identity() {
        let (scoped, _) = scoped_vrf_identity(&qaddr(77)).unwrap();
        assert_ne!(scoped, DEPRECATED_GLOBAL_VRF_PROGRAM_IDENTITY);
    }

    #[test]
    fn callback_randomness_decode_skips_discriminator() {
        let mut data = Vec::new();
        data.extend_from_slice(&[1, 2, 3, 4]);
        data.extend_from_slice(&[9u8; 32]);
        data.extend_from_slice(&[7, 8]);

        let randomness = decode_callback_randomness(&data, 4).unwrap();
        assert_eq!(randomness, [9u8; 32]);
        assert!(decode_callback_randomness(&data[..20], 4).is_err());
    }

    #[test]
    fn pda_derivations_match_solana_program_reference() {
        let callback_program_id = qaddr(88);
        let (ours_identity, ours_identity_bump) =
            program_identity_pda(&callback_program_id).unwrap();
        let (expected_identity, expected_identity_bump) = Pubkey::find_program_address(
            &[IDENTITY_SEED],
            &Pubkey::new_from_array(callback_program_id.to_bytes()),
        );
        assert_eq!(ours_identity.as_ref(), expected_identity.as_ref());
        assert_eq!(ours_identity_bump, expected_identity_bump);

        let (ours_scoped, ours_scoped_bump) = scoped_vrf_identity(&callback_program_id).unwrap();
        let (expected_scoped, expected_scoped_bump) = Pubkey::find_program_address(
            &[IDENTITY_SEED, callback_program_id.as_ref()],
            &Pubkey::new_from_array(VRF_PROGRAM_ID.to_bytes()),
        );
        assert_eq!(ours_scoped.as_ref(), expected_scoped.as_ref());
        assert_eq!(ours_scoped_bump, expected_scoped_bump);
    }
}
