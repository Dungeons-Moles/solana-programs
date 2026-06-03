use {
    crate::{constants::*, errors::MarketplaceError},
    quasar_lang::{
        cpi::{CpiDynamic, CpiSignerSeeds},
        prelude::*,
    },
};

const MPL_CREATE_V1: u8 = 0;
const MPL_ADD_PLUGIN_V1: u8 = 2;
const MPL_REMOVE_PLUGIN_V1: u8 = 4;
const MPL_APPROVE_PLUGIN_AUTHORITY_V1: u8 = 8;
const MPL_TRANSFER_V1: u8 = 14;

const DATA_STATE_ACCOUNT: u8 = 0;
const PLUGIN_TRANSFER_DELEGATE: u8 = 3;
const PLUGIN_AUTHORITY_ADDRESS: u8 = 3;

pub struct CreateV1Accounts<'a> {
    pub program: &'a AccountView,
    pub asset: &'a AccountView,
    pub collection: Option<&'a AccountView>,
    pub authority: Option<&'a AccountView>,
    pub payer: &'a AccountView,
    pub owner: Option<&'a AccountView>,
    pub update_authority: Option<&'a AccountView>,
    pub system_program: &'a AccountView,
    pub log_wrapper: Option<&'a AccountView>,
}

pub struct AddPluginV1Accounts<'a> {
    pub program: &'a AccountView,
    pub asset: &'a AccountView,
    pub collection: Option<&'a AccountView>,
    pub payer: &'a AccountView,
    pub authority: Option<&'a AccountView>,
    pub system_program: &'a AccountView,
}

pub struct ApprovePluginAuthorityV1Accounts<'a> {
    pub program: &'a AccountView,
    pub asset: &'a AccountView,
    pub collection: Option<&'a AccountView>,
    pub payer: &'a AccountView,
    pub authority: Option<&'a AccountView>,
    pub system_program: &'a AccountView,
}

pub struct RemovePluginV1Accounts<'a> {
    pub program: &'a AccountView,
    pub asset: &'a AccountView,
    pub collection: Option<&'a AccountView>,
    pub payer: &'a AccountView,
    pub authority: Option<&'a AccountView>,
    pub system_program: &'a AccountView,
}

pub struct TransferV1Accounts<'a> {
    pub program: &'a AccountView,
    pub asset: &'a AccountView,
    pub collection: Option<&'a AccountView>,
    pub payer: &'a AccountView,
    pub authority: Option<&'a AccountView>,
    pub new_owner: &'a AccountView,
}

#[inline(always)]
fn write_u8(buf: &mut [u8], cursor: &mut usize, value: u8) -> Result<(), ProgramError> {
    if *cursor >= buf.len() {
        return Err(ProgramError::InvalidInstructionData);
    }
    buf[*cursor] = value;
    *cursor += 1;
    Ok(())
}

#[inline(always)]
fn write_bytes(buf: &mut [u8], cursor: &mut usize, bytes: &[u8]) -> Result<(), ProgramError> {
    let end = cursor
        .checked_add(bytes.len())
        .ok_or(MarketplaceError::ArithmeticOverflow)?;
    if end > buf.len() {
        return Err(ProgramError::InvalidInstructionData);
    }
    buf[*cursor..end].copy_from_slice(bytes);
    *cursor = end;
    Ok(())
}

#[inline(always)]
fn write_string(
    buf: &mut [u8],
    cursor: &mut usize,
    value: &str,
    max_len: usize,
) -> Result<(), ProgramError> {
    let bytes = value.as_bytes();
    if bytes.len() > max_len {
        return Err(MarketplaceError::InvalidAccountData.into());
    }
    let len = (bytes.len() as u32).to_le_bytes();
    write_bytes(buf, cursor, &len)?;
    write_bytes(buf, cursor, bytes)
}

#[inline(always)]
fn push_optional_account<'a, const A: usize, const D: usize>(
    cpi: &mut CpiDynamic<'a, A, D>,
    account: Option<&'a AccountView>,
    sentinel: &'a AccountView,
    is_signer: bool,
    is_writable: bool,
) -> Result<(), ProgramError> {
    let view = account.unwrap_or(sentinel);
    let signer = account.is_some() && is_signer;
    let writable = account.is_some() && is_writable;
    cpi.push_account(view, signer, writable)
}

pub fn create_v1<S: CpiSignerSeeds + ?Sized>(
    accounts: CreateV1Accounts,
    name: impl AsRef<str>,
    uri: impl AsRef<str>,
    signer_seeds: &S,
) -> Result<(), ProgramError> {
    let mut cpi = CpiDynamic::<8, MPL_CREATE_MAX_DATA>::new(accounts.program.address());
    cpi.push_account(accounts.asset, true, true)?;
    push_optional_account(&mut cpi, accounts.collection, accounts.program, false, true)?;
    push_optional_account(&mut cpi, accounts.authority, accounts.program, true, false)?;
    cpi.push_account(accounts.payer, true, true)?;
    push_optional_account(&mut cpi, accounts.owner, accounts.program, false, false)?;
    push_optional_account(
        &mut cpi,
        accounts.update_authority,
        accounts.program,
        false,
        false,
    )?;
    cpi.push_account(accounts.system_program, false, false)?;
    push_optional_account(
        &mut cpi,
        accounts.log_wrapper,
        accounts.program,
        false,
        false,
    )?;

    let mut data = [0u8; MPL_CREATE_MAX_DATA];
    let mut cursor = 0usize;
    write_u8(&mut data, &mut cursor, MPL_CREATE_V1)?;
    write_u8(&mut data, &mut cursor, DATA_STATE_ACCOUNT)?;
    write_string(&mut data, &mut cursor, name.as_ref(), MAX_MPL_NAME_LENGTH)?;
    write_string(&mut data, &mut cursor, uri.as_ref(), MAX_MPL_URI_LENGTH)?;
    write_u8(&mut data, &mut cursor, 0)?;
    cpi.set_data(&data[..cursor])?;
    cpi.invoke_signed(signer_seeds)
}

pub fn add_transfer_delegate_plugin(accounts: AddPluginV1Accounts) -> Result<(), ProgramError> {
    let mut cpi = CpiDynamic::<6, 4>::new(accounts.program.address());
    cpi.push_account(accounts.asset, false, true)?;
    push_optional_account(&mut cpi, accounts.collection, accounts.program, false, true)?;
    cpi.push_account(accounts.payer, true, true)?;
    push_optional_account(&mut cpi, accounts.authority, accounts.program, true, false)?;
    cpi.push_account(accounts.system_program, false, false)?;
    cpi.push_account(accounts.program, false, false)?;

    let data = [
        MPL_ADD_PLUGIN_V1,
        PLUGIN_TRANSFER_DELEGATE,
        0, // init_authority: None
    ];
    cpi.set_data(&data)?;
    cpi.invoke()
}

pub fn approve_transfer_delegate_authority(
    accounts: ApprovePluginAuthorityV1Accounts,
    new_authority: &Address,
) -> Result<(), ProgramError> {
    let mut cpi = CpiDynamic::<6, 40>::new(accounts.program.address());
    cpi.push_account(accounts.asset, false, true)?;
    push_optional_account(&mut cpi, accounts.collection, accounts.program, false, true)?;
    cpi.push_account(accounts.payer, true, true)?;
    push_optional_account(&mut cpi, accounts.authority, accounts.program, true, false)?;
    cpi.push_account(accounts.system_program, false, false)?;
    cpi.push_account(accounts.program, false, false)?;

    let mut data = [0u8; 40];
    let mut cursor = 0usize;
    write_u8(&mut data, &mut cursor, MPL_APPROVE_PLUGIN_AUTHORITY_V1)?;
    write_u8(&mut data, &mut cursor, PLUGIN_TRANSFER_DELEGATE)?;
    write_u8(&mut data, &mut cursor, PLUGIN_AUTHORITY_ADDRESS)?;
    write_bytes(&mut data, &mut cursor, new_authority.as_ref())?;
    cpi.set_data(&data[..cursor])?;
    cpi.invoke()
}

pub fn remove_transfer_delegate_plugin(
    accounts: RemovePluginV1Accounts,
) -> Result<(), ProgramError> {
    let mut cpi = CpiDynamic::<6, 4>::new(accounts.program.address());
    cpi.push_account(accounts.asset, false, true)?;
    push_optional_account(&mut cpi, accounts.collection, accounts.program, false, true)?;
    cpi.push_account(accounts.payer, true, true)?;
    push_optional_account(&mut cpi, accounts.authority, accounts.program, true, false)?;
    cpi.push_account(accounts.system_program, false, false)?;
    cpi.push_account(accounts.program, false, false)?;

    let data = [MPL_REMOVE_PLUGIN_V1, PLUGIN_TRANSFER_DELEGATE];
    cpi.set_data(&data)?;
    cpi.invoke()
}

pub fn transfer_v1<S: CpiSignerSeeds + ?Sized>(
    accounts: TransferV1Accounts,
    signer_seeds: &S,
) -> Result<(), ProgramError> {
    let mut cpi = CpiDynamic::<7, 4>::new(accounts.program.address());
    cpi.push_account(accounts.asset, false, true)?;
    push_optional_account(
        &mut cpi,
        accounts.collection,
        accounts.program,
        false,
        false,
    )?;
    cpi.push_account(accounts.payer, true, true)?;
    push_optional_account(&mut cpi, accounts.authority, accounts.program, true, false)?;
    cpi.push_account(accounts.new_owner, false, false)?;
    cpi.push_account(accounts.program, false, false)?;
    cpi.push_account(accounts.program, false, false)?;

    let data = [MPL_TRANSFER_V1, 0];
    cpi.set_data(&data)?;
    cpi.invoke_signed(signer_seeds)
}
