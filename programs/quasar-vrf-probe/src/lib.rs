#![no_std]

#[cfg(test)]
extern crate std;

use er_compat::quasar_vrf::{
    commit_and_undelegate_account, delegate_account_to_ephemeral,
    delegate_buffer_pda_from_delegated_account_and_owner_program,
    delegation_metadata_pda_from_delegated_account, delegation_record_pda_from_delegated_account,
    process_external_undelegate, request_scoped_ephemeral_vrf_with_identity_bump_for_queue,
    verify_scoped_vrf_identity, CallbackAccountMeta, CommitAndUndelegateAccounts,
    DelegateAccountAccounts, ExternalUndelegateAccounts, RequestRandomness,
    RequestRandomnessAccounts, DELEGATION_PROGRAM_ID, ER_ORACLE_QUEUE, LOCAL_SERVICEABLE_ER_QUEUE,
    MAGIC_CONTEXT_ID, MAGIC_PROGRAM_ID,
};
use quasar_lang::prelude::*;

pub mod state;

use state::VrfProbe;

declare_id!("5s8adsH9jmPrSZRSRyDu5vBXp3X5ySm5SXg673Yvrm94");

const CALLBACK_VRF_DISCRIMINATOR: [u8; 8] = [3, 0, 0, 0, 0, 0, 0, 0];
const REQUEST_VRF_MAX_DATA: usize =
    RequestRandomness::serialized_size_for(CALLBACK_VRF_DISCRIMINATOR.len(), 1, 0);
const CALLBACK_UNDELEGATE_ARGS_LEN: usize = 4 + 4 + VrfProbe::SEED_PREFIX.len() + 4 + 32;
const PROBE_OWNER_OFFSET: usize = 8;
const PROBE_REQUEST_COUNT_OFFSET: usize = 40;
const PROBE_FULFILLED_COUNT_OFFSET: usize = 48;
const PROBE_LAST_REQUEST_SEED_OFFSET: usize = 56;
const PROBE_LAST_RANDOMNESS_OFFSET: usize = 88;
const PROBE_LAST_DIE_ROLL_OFFSET: usize = 120;
const PROBE_CALLBACK_VERIFIED_OFFSET: usize = 121;
const PROBE_MIN_DATA_LEN: usize = 123;

fn expected_oracle_queue() -> Address {
    if cfg!(feature = "local-vrf-queue") {
        LOCAL_SERVICEABLE_ER_QUEUE
    } else {
        ER_ORACLE_QUEUE
    }
}

fn check_probe_data(data: &[u8]) -> Result<(), ProgramError> {
    if data.len() < PROBE_MIN_DATA_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    if data[..8] != state::VRF_PROBE_DISCRIMINATOR {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

fn read_probe_owner(probe: &UncheckedAccount) -> Result<Address, ProgramError> {
    let data = probe.to_account_view().try_borrow()?;
    check_probe_data(&data)?;
    let mut owner = [0; 32];
    owner.copy_from_slice(&data[PROBE_OWNER_OFFSET..PROBE_OWNER_OFFSET + 32]);
    Ok(Address::new_from_array(owner))
}

fn read_probe_u64(probe: &UncheckedAccount, offset: usize) -> Result<u64, ProgramError> {
    let data = probe.to_account_view().try_borrow()?;
    check_probe_data(&data)?;
    let mut value = [0; 8];
    value.copy_from_slice(&data[offset..offset + 8]);
    Ok(u64::from_le_bytes(value))
}

fn write_probe_bytes(
    probe: &mut UncheckedAccount,
    offset: usize,
    bytes: &[u8],
) -> Result<(), ProgramError> {
    probe.write_bytes(offset, bytes)
}

fn write_probe_u64(
    probe: &mut UncheckedAccount,
    offset: usize,
    value: u64,
) -> Result<(), ProgramError> {
    write_probe_bytes(probe, offset, &value.to_le_bytes())
}

#[program(no_entrypoint)]
mod quasar_vrf_probe {
    use super::*;

    #[instruction(discriminator = [1, 0, 0, 0, 0, 0, 0, 0])]
    pub fn initialize_probe(ctx: Ctx<InitializeProbe>) -> Result<(), ProgramError> {
        ctx.accounts.probe.set_inner(state::VrfProbeInner {
            owner: *ctx.accounts.owner.address(),
            request_count: 0,
            fulfilled_count: 0,
            last_request_seed: [0; 32],
            last_randomness: [0; 32],
            last_die_roll: 0,
            callback_verified: false,
            bump: ctx.bumps.probe,
        });
        Ok(())
    }

    #[instruction(discriminator = [2, 0, 0, 0, 0, 0, 0, 0])]
    pub fn request_vrf(
        ctx: Ctx<RequestVrf>,
        caller_seed: [u8; 32],
        high_priority: bool,
    ) -> Result<(), ProgramError> {
        log("quasar-vrf-probe: request_vrf start");
        let (expected_probe, _) = Address::find_program_address(
            &[VrfProbe::SEED_PREFIX, ctx.accounts.payer.address().as_ref()],
            &ID,
        );
        if *ctx.accounts.probe.address() != expected_probe {
            return Err(ProgramError::InvalidSeeds);
        }
        let expected_queue = expected_oracle_queue();
        if ctx.accounts.oracle_queue.address() != &expected_queue {
            return Err(ProgramError::InvalidArgument);
        }
        if read_probe_owner(&ctx.accounts.probe)? != *ctx.accounts.payer.address() {
            return Err(ProgramError::InvalidAccountData);
        }

        let payer = ctx.accounts.payer.to_account_view();
        let program_identity = ctx.accounts.program_identity.to_account_view();
        let oracle_queue = ctx.accounts.oracle_queue.to_account_view();
        let system_program = ctx.accounts.system_program.to_account_view();
        let slot_hashes = ctx.accounts.slot_hashes.to_account_view();
        let vrf_program = ctx.accounts.vrf_program.to_account_view();
        let probe = ctx.accounts.probe.to_account_view();

        let callback_metas = [CallbackAccountMeta {
            address: *probe.address(),
            is_signer: false,
            is_writable: true,
        }];
        let request = RequestRandomness {
            high_priority,
            caller_seed,
            callback_program_id: &ID,
            callback_discriminator: &CALLBACK_VRF_DISCRIMINATOR,
            callback_accounts_metas: &callback_metas,
            callback_args: &[],
        };
        let request_accounts = RequestRandomnessAccounts {
            payer: &payer,
            program_identity: &program_identity,
            oracle_queue: &oracle_queue,
            system_program: &system_program,
            slot_hashes: &slot_hashes,
            vrf_program: &vrf_program,
        };

        let (_, identity_bump) = er_compat::quasar_vrf::program_identity_pda(&ID)?;
        log("quasar-vrf-probe: request_vrf before cpi");
        request_scoped_ephemeral_vrf_with_identity_bump_for_queue::<REQUEST_VRF_MAX_DATA>(
            &request_accounts,
            &request,
            identity_bump,
            &expected_queue,
        )?;
        log("quasar-vrf-probe: request_vrf after cpi");

        let next_count =
            read_probe_u64(&ctx.accounts.probe, PROBE_REQUEST_COUNT_OFFSET)?.saturating_add(1);
        write_probe_bytes(
            &mut ctx.accounts.probe,
            PROBE_LAST_REQUEST_SEED_OFFSET,
            &caller_seed,
        )?;
        write_probe_bytes(
            &mut ctx.accounts.probe,
            PROBE_CALLBACK_VERIFIED_OFFSET,
            &[0],
        )?;
        write_probe_u64(
            &mut ctx.accounts.probe,
            PROBE_REQUEST_COUNT_OFFSET,
            next_count,
        )?;
        Ok(())
    }

    #[instruction(discriminator = [6, 0, 0, 0, 0, 0, 0, 0])]
    pub fn request_vrf_fixed(ctx: Ctx<RequestVrf>) -> Result<(), ProgramError> {
        let caller_seed = [7; 32];
        let high_priority = true;

        log("quasar-vrf-probe: request_vrf_fixed start");
        let (expected_probe, _) = Address::find_program_address(
            &[VrfProbe::SEED_PREFIX, ctx.accounts.payer.address().as_ref()],
            &ID,
        );
        if *ctx.accounts.probe.address() != expected_probe {
            return Err(ProgramError::InvalidSeeds);
        }
        let expected_queue = expected_oracle_queue();
        if ctx.accounts.oracle_queue.address() != &expected_queue {
            return Err(ProgramError::InvalidArgument);
        }
        if read_probe_owner(&ctx.accounts.probe)? != *ctx.accounts.payer.address() {
            return Err(ProgramError::InvalidAccountData);
        }

        let payer = ctx.accounts.payer.to_account_view();
        let program_identity = ctx.accounts.program_identity.to_account_view();
        let oracle_queue = ctx.accounts.oracle_queue.to_account_view();
        let system_program = ctx.accounts.system_program.to_account_view();
        let slot_hashes = ctx.accounts.slot_hashes.to_account_view();
        let vrf_program = ctx.accounts.vrf_program.to_account_view();
        let probe = ctx.accounts.probe.to_account_view();

        let callback_metas = [CallbackAccountMeta {
            address: *probe.address(),
            is_signer: false,
            is_writable: true,
        }];
        let request = RequestRandomness {
            high_priority,
            caller_seed,
            callback_program_id: &ID,
            callback_discriminator: &CALLBACK_VRF_DISCRIMINATOR,
            callback_accounts_metas: &callback_metas,
            callback_args: &[],
        };
        let request_accounts = RequestRandomnessAccounts {
            payer: &payer,
            program_identity: &program_identity,
            oracle_queue: &oracle_queue,
            system_program: &system_program,
            slot_hashes: &slot_hashes,
            vrf_program: &vrf_program,
        };

        let (_, identity_bump) = er_compat::quasar_vrf::program_identity_pda(&ID)?;
        log("quasar-vrf-probe: request_vrf_fixed before cpi");
        request_scoped_ephemeral_vrf_with_identity_bump_for_queue::<REQUEST_VRF_MAX_DATA>(
            &request_accounts,
            &request,
            identity_bump,
            &expected_queue,
        )?;
        log("quasar-vrf-probe: request_vrf_fixed after cpi");

        let next_count =
            read_probe_u64(&ctx.accounts.probe, PROBE_REQUEST_COUNT_OFFSET)?.saturating_add(1);
        write_probe_bytes(
            &mut ctx.accounts.probe,
            PROBE_LAST_REQUEST_SEED_OFFSET,
            &caller_seed,
        )?;
        write_probe_bytes(
            &mut ctx.accounts.probe,
            PROBE_CALLBACK_VERIFIED_OFFSET,
            &[0],
        )?;
        write_probe_u64(
            &mut ctx.accounts.probe,
            PROBE_REQUEST_COUNT_OFFSET,
            next_count,
        )?;
        Ok(())
    }

    #[instruction(discriminator = [7, 0, 0, 0, 0, 0, 0, 0])]
    pub fn ping(_ctx: Ctx<Ping>) -> Result<(), ProgramError> {
        log("quasar-vrf-probe: ping");
        Ok(())
    }

    #[instruction(discriminator = [3, 0, 0, 0, 0, 0, 0, 0])]
    pub fn callback_vrf(ctx: Ctx<CallbackVrf>, randomness: [u8; 32]) -> Result<(), ProgramError> {
        let scoped_vrf_identity = ctx.accounts.scoped_vrf_identity.to_account_view();
        verify_scoped_vrf_identity(&ID, &scoped_vrf_identity)?;

        let die_roll = (randomness[0] % 6).saturating_add(1);
        let next_count =
            read_probe_u64(&ctx.accounts.probe, PROBE_FULFILLED_COUNT_OFFSET)?.saturating_add(1);
        write_probe_bytes(
            &mut ctx.accounts.probe,
            PROBE_LAST_RANDOMNESS_OFFSET,
            &randomness,
        )?;
        write_probe_bytes(
            &mut ctx.accounts.probe,
            PROBE_LAST_DIE_ROLL_OFFSET,
            &[die_roll],
        )?;
        write_probe_bytes(
            &mut ctx.accounts.probe,
            PROBE_CALLBACK_VERIFIED_OFFSET,
            &[1],
        )?;
        write_probe_u64(
            &mut ctx.accounts.probe,
            PROBE_FULFILLED_COUNT_OFFSET,
            next_count,
        )?;
        Ok(())
    }

    #[instruction(discriminator = [4, 0, 0, 0, 0, 0, 0, 0])]
    pub fn delegate_probe(ctx: Ctx<DelegateProbe>) -> Result<(), ProgramError> {
        if ctx.accounts.probe.owner != *ctx.accounts.payer.address() {
            return Err(ProgramError::InvalidAccountData);
        }
        if *ctx.accounts.owner_program.address() != ID {
            return Err(ProgramError::IncorrectProgramId);
        }
        if *ctx.accounts.delegation_program.address() != DELEGATION_PROGRAM_ID {
            return Err(ProgramError::IncorrectProgramId);
        }

        let probe_address = *ctx.accounts.probe.address();
        if *ctx.accounts.buffer.address()
            != delegate_buffer_pda_from_delegated_account_and_owner_program(&probe_address, &ID)
        {
            return Err(ProgramError::InvalidSeeds);
        }
        if *ctx.accounts.delegation_record.address()
            != delegation_record_pda_from_delegated_account(&probe_address)
        {
            return Err(ProgramError::InvalidSeeds);
        }
        if *ctx.accounts.delegation_metadata.address()
            != delegation_metadata_pda_from_delegated_account(&probe_address)
        {
            return Err(ProgramError::InvalidSeeds);
        }

        let payer_address = *ctx.accounts.payer.address();
        let seeds = [VrfProbe::SEED_PREFIX, payer_address.as_ref()];
        let accounts = DelegateAccountAccounts {
            payer: ctx.accounts.payer.to_account_view(),
            delegated_account: ctx.accounts.probe.to_account_view(),
            owner_program: ctx.accounts.owner_program.to_account_view(),
            buffer: ctx.accounts.buffer.to_account_view(),
            delegation_record: ctx.accounts.delegation_record.to_account_view(),
            delegation_metadata: ctx.accounts.delegation_metadata.to_account_view(),
            system_program: ctx.accounts.system_program.to_account_view(),
        };
        delegate_account_to_ephemeral(
            &accounts,
            &seeds,
            ctx.accounts.probe.bump,
            Some(*ctx.accounts.validator.address()),
        )
    }

    #[instruction(discriminator = [5, 0, 0, 0, 0, 0, 0, 0])]
    pub fn undelegate_probe(ctx: Ctx<UndelegateProbe>) -> Result<(), ProgramError> {
        let accounts = CommitAndUndelegateAccounts {
            payer: ctx.accounts.payer.to_account_view(),
            magic_context: ctx.accounts.magic_context.to_account_view(),
            magic_program: ctx.accounts.magic_program.to_account_view(),
            delegated_account: ctx.accounts.probe.to_account_view(),
        };
        commit_and_undelegate_account(&accounts)
    }

    #[instruction(discriminator = [196, 28, 41, 206, 48, 37, 51, 167])]
    pub fn callback_undelegate_probe(
        ctx: Ctx<CallbackUndelegateProbe>,
        callback_args: [u8; CALLBACK_UNDELEGATE_ARGS_LEN],
    ) -> Result<(), ProgramError> {
        let accounts = ExternalUndelegateAccounts {
            delegated_account: ctx.accounts.delegated_account.to_account_view(),
            buffer: ctx.accounts.buffer.to_account_view(),
            payer: ctx.accounts.payer.to_account_view(),
        };
        process_external_undelegate(&accounts, &ID, &callback_args)
    }
}

#[cfg(all(
    not(feature = "no-entrypoint"),
    any(target_os = "solana", target_arch = "bpf")
))]
#[unsafe(no_mangle)]
#[allow(unexpected_cfgs)]
pub unsafe extern "C" fn entrypoint(ptr: *mut u8) -> u64 {
    const UNINIT: core::mem::MaybeUninit<pinocchio::AccountView> =
        core::mem::MaybeUninit::<pinocchio::AccountView>::uninit();
    let mut accounts = [UNINIT; pinocchio::MAX_TX_ACCOUNTS];
    let (_, _, instruction_data) = unsafe {
        pinocchio::entrypoint::deserialize::<{ pinocchio::MAX_TX_ACCOUNTS }>(ptr, &mut accounts)
    };

    match quasar_vrf_probe::__dispatch(ptr, instruction_data) {
        Ok(_) => 0,
        Err(error) => error.into(),
    }
}

#[derive(Accounts)]
pub struct InitializeProbe {
    #[account(mut)]
    pub owner: Signer,
    #[account(
        mut,
        init,
        payer = owner,
        address = VrfProbe::seeds(owner.address())
    )]
    pub probe: Account<VrfProbe>,
    pub rent: Sysvar<Rent>,
    pub system_program: Program<SystemProgram>,
}

#[derive(Accounts)]
pub struct RequestVrf {
    #[account(mut)]
    pub payer: Signer,
    #[account(mut)]
    pub probe: UncheckedAccount,
    #[account(mut)]
    pub oracle_queue: UncheckedAccount,
    pub program_identity: UncheckedAccount,
    pub vrf_program: UncheckedAccount,
    pub slot_hashes: UncheckedAccount,
    pub system_program: UncheckedAccount,
}

#[derive(Accounts)]
pub struct CallbackVrf {
    pub scoped_vrf_identity: UncheckedAccount,
    #[account(mut)]
    pub probe: UncheckedAccount,
}

#[derive(Accounts)]
pub struct Ping {}

#[derive(Accounts)]
pub struct DelegateProbe {
    #[account(mut)]
    pub payer: Signer,
    #[account(
        mut,
        address = VrfProbe::seeds(payer.address())
    )]
    pub probe: Account<VrfProbe>,
    pub owner_program: UncheckedAccount,
    #[account(mut)]
    pub buffer: UncheckedAccount,
    #[account(mut)]
    pub delegation_record: UncheckedAccount,
    #[account(mut)]
    pub delegation_metadata: UncheckedAccount,
    pub system_program: Program<SystemProgram>,
    pub delegation_program: UncheckedAccount,
    pub validator: UncheckedAccount,
}

#[derive(Accounts)]
pub struct UndelegateProbe {
    pub payer: Signer,
    #[account(mut)]
    pub probe: UncheckedAccount,
    #[account(mut, address = MAGIC_CONTEXT_ID)]
    pub magic_context: UncheckedAccount,
    #[account(address = MAGIC_PROGRAM_ID)]
    pub magic_program: UncheckedAccount,
}

#[derive(Accounts)]
pub struct CallbackUndelegateProbe {
    #[account(mut)]
    pub delegated_account: UncheckedAccount,
    #[account(mut)]
    pub buffer: UncheckedAccount,
    #[account(mut)]
    pub payer: UncheckedAccount,
    pub system_program: Program<SystemProgram>,
}
