use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::{invoke, invoke_signed};
use anchor_lang::solana_program::pubkey;

pub const MPL_CORE_ID: Pubkey = pubkey!("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d");

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq)]
pub enum DataState {
    AccountState,
    LedgerState,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq)]
pub enum PluginAuthority {
    None,
    Owner,
    UpdateAuthority,
    Address { address: Pubkey },
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq, PartialOrd, Hash)]
pub enum PluginType {
    Royalties,
    FreezeDelegate,
    BurnDelegate,
    TransferDelegate,
    UpdateDelegate,
    PermanentFreezeDelegate,
    Attributes,
    PermanentTransferDelegate,
    PermanentBurnDelegate,
    Edition,
    MasterEdition,
    AddBlocker,
    ImmutableMetadata,
    VerifiedCreators,
    Autograph,
    BubblegumV2,
    FreezeExecute,
    PermanentFreezeExecute,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct Royalties {}
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct FreezeDelegate {}
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct BurnDelegate {}
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct TransferDelegate {}
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct UpdateDelegate {}
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct PermanentFreezeDelegate {}
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct Attributes {}
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct PermanentTransferDelegate {}
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct PermanentBurnDelegate {}
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct Edition {}
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct MasterEdition {}
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct AddBlocker {}
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct ImmutableMetadata {}
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct VerifiedCreators {}
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct Autograph {}
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct BubblegumV2 {}
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct FreezeExecute {}
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct PermanentFreezeExecute {}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq)]
pub enum Plugin {
    Royalties(Royalties),
    FreezeDelegate(FreezeDelegate),
    BurnDelegate(BurnDelegate),
    TransferDelegate(TransferDelegate),
    UpdateDelegate(UpdateDelegate),
    PermanentFreezeDelegate(PermanentFreezeDelegate),
    Attributes(Attributes),
    PermanentTransferDelegate(PermanentTransferDelegate),
    PermanentBurnDelegate(PermanentBurnDelegate),
    Edition(Edition),
    MasterEdition(MasterEdition),
    AddBlocker(AddBlocker),
    ImmutableMetadata(ImmutableMetadata),
    VerifiedCreators(VerifiedCreators),
    Autograph(Autograph),
    BubblegumV2(BubblegumV2),
    FreezeExecute(FreezeExecute),
    PermanentFreezeExecute(PermanentFreezeExecute),
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq)]
pub struct PluginAuthorityPair {
    pub plugin: Plugin,
    pub authority: Option<PluginAuthority>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct CompressionProof {}

#[derive(AnchorSerialize, AnchorDeserialize)]
struct CreateV1InstructionData {
    discriminator: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq)]
struct CreateV1InstructionArgs {
    data_state: DataState,
    name: String,
    uri: String,
    plugins: Option<Vec<PluginAuthorityPair>>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
struct AddPluginV1InstructionData {
    discriminator: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq)]
struct AddPluginV1InstructionArgs {
    plugin: Plugin,
    init_authority: Option<PluginAuthority>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
struct ApprovePluginAuthorityV1InstructionData {
    discriminator: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq)]
struct ApprovePluginAuthorityV1InstructionArgs {
    plugin_type: PluginType,
    new_authority: PluginAuthority,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
struct RemovePluginV1InstructionData {
    discriminator: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq)]
struct RemovePluginV1InstructionArgs {
    plugin_type: PluginType,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
struct TransferV1InstructionData {
    discriminator: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq)]
struct TransferV1InstructionArgs {
    compression_proof: Option<CompressionProof>,
}

pub struct CreateV1Accounts<'a, 'info> {
    pub program: &'a AccountInfo<'info>,
    pub asset: &'a AccountInfo<'info>,
    pub collection: Option<&'a AccountInfo<'info>>,
    pub authority: Option<&'a AccountInfo<'info>>,
    pub payer: &'a AccountInfo<'info>,
    pub owner: Option<&'a AccountInfo<'info>>,
    pub update_authority: Option<&'a AccountInfo<'info>>,
    pub system_program: &'a AccountInfo<'info>,
    pub log_wrapper: Option<&'a AccountInfo<'info>>,
}

pub fn create_v1<'a, 'info>(
    accounts: CreateV1Accounts<'a, 'info>,
    name: String,
    uri: String,
    signers_seeds: &[&[&[u8]]],
) -> Result<()> {
    let mut data = CreateV1InstructionData { discriminator: 0 }.try_to_vec()?;
    data.extend_from_slice(
        &CreateV1InstructionArgs {
            data_state: DataState::AccountState,
            name,
            uri,
            plugins: None,
        }
        .try_to_vec()?,
    );

    let mut metas = vec![AccountMeta::new(*accounts.asset.key, true)];
    push_optional_meta(&mut metas, accounts.collection, true, false);
    push_optional_meta(&mut metas, accounts.authority, false, true);
    metas.push(AccountMeta::new(*accounts.payer.key, true));
    push_optional_meta(&mut metas, accounts.owner, false, false);
    push_optional_meta(&mut metas, accounts.update_authority, false, false);
    metas.push(AccountMeta::new_readonly(*accounts.system_program.key, false));
    push_optional_meta(&mut metas, accounts.log_wrapper, false, false);

    let ix = Instruction {
        program_id: MPL_CORE_ID,
        accounts: metas,
        data,
    };

    let mut infos = vec![
        accounts.program.clone(),
        accounts.asset.clone(),
    ];
    push_optional_info(&mut infos, accounts.collection);
    push_optional_info(&mut infos, accounts.authority);
    infos.push(accounts.payer.clone());
    push_optional_info(&mut infos, accounts.owner);
    push_optional_info(&mut infos, accounts.update_authority);
    infos.push(accounts.system_program.clone());
    push_optional_info(&mut infos, accounts.log_wrapper);

    invoke_mpl(ix, &infos, signers_seeds)
}

pub struct AddPluginV1Accounts<'a, 'info> {
    pub program: &'a AccountInfo<'info>,
    pub asset: &'a AccountInfo<'info>,
    pub collection: Option<&'a AccountInfo<'info>>,
    pub payer: &'a AccountInfo<'info>,
    pub authority: Option<&'a AccountInfo<'info>>,
    pub system_program: &'a AccountInfo<'info>,
}

pub fn add_transfer_delegate_plugin<'a, 'info>(
    accounts: AddPluginV1Accounts<'a, 'info>,
) -> Result<()> {
    let mut data = AddPluginV1InstructionData { discriminator: 2 }.try_to_vec()?;
    data.extend_from_slice(
        &AddPluginV1InstructionArgs {
            plugin: Plugin::TransferDelegate(TransferDelegate {}),
            init_authority: None,
        }
        .try_to_vec()?,
    );

    let mut metas = vec![AccountMeta::new(*accounts.asset.key, false)];
    push_optional_meta(&mut metas, accounts.collection, true, false);
    metas.push(AccountMeta::new(*accounts.payer.key, true));
    push_optional_meta(&mut metas, accounts.authority, false, true);
    metas.push(AccountMeta::new_readonly(*accounts.system_program.key, false));
    metas.push(AccountMeta::new_readonly(MPL_CORE_ID, false));

    let ix = Instruction {
        program_id: MPL_CORE_ID,
        accounts: metas,
        data,
    };

    let mut infos = vec![accounts.program.clone(), accounts.asset.clone()];
    push_optional_info(&mut infos, accounts.collection);
    infos.push(accounts.payer.clone());
    push_optional_info(&mut infos, accounts.authority);
    infos.push(accounts.system_program.clone());

    invoke(&ix, &infos).map_err(Into::into)
}

pub struct ApprovePluginAuthorityV1Accounts<'a, 'info> {
    pub program: &'a AccountInfo<'info>,
    pub asset: &'a AccountInfo<'info>,
    pub collection: Option<&'a AccountInfo<'info>>,
    pub payer: &'a AccountInfo<'info>,
    pub authority: Option<&'a AccountInfo<'info>>,
    pub system_program: &'a AccountInfo<'info>,
}

pub fn approve_plugin_authority<'a, 'info>(
    accounts: ApprovePluginAuthorityV1Accounts<'a, 'info>,
    plugin_type: PluginType,
    new_authority: PluginAuthority,
) -> Result<()> {
    let mut data =
        ApprovePluginAuthorityV1InstructionData { discriminator: 8 }.try_to_vec()?;
    data.extend_from_slice(
        &ApprovePluginAuthorityV1InstructionArgs {
            plugin_type,
            new_authority,
        }
        .try_to_vec()?,
    );

    let mut metas = vec![AccountMeta::new(*accounts.asset.key, false)];
    push_optional_meta(&mut metas, accounts.collection, true, false);
    metas.push(AccountMeta::new(*accounts.payer.key, true));
    push_optional_meta(&mut metas, accounts.authority, false, true);
    metas.push(AccountMeta::new_readonly(*accounts.system_program.key, false));
    metas.push(AccountMeta::new_readonly(MPL_CORE_ID, false));

    let ix = Instruction {
        program_id: MPL_CORE_ID,
        accounts: metas,
        data,
    };

    let mut infos = vec![accounts.program.clone(), accounts.asset.clone()];
    push_optional_info(&mut infos, accounts.collection);
    infos.push(accounts.payer.clone());
    push_optional_info(&mut infos, accounts.authority);
    infos.push(accounts.system_program.clone());

    invoke(&ix, &infos).map_err(Into::into)
}

pub struct RemovePluginV1Accounts<'a, 'info> {
    pub program: &'a AccountInfo<'info>,
    pub asset: &'a AccountInfo<'info>,
    pub collection: Option<&'a AccountInfo<'info>>,
    pub payer: &'a AccountInfo<'info>,
    pub authority: Option<&'a AccountInfo<'info>>,
    pub system_program: &'a AccountInfo<'info>,
}

pub fn remove_plugin<'a, 'info>(
    accounts: RemovePluginV1Accounts<'a, 'info>,
    plugin_type: PluginType,
) -> Result<()> {
    let mut data = RemovePluginV1InstructionData { discriminator: 4 }.try_to_vec()?;
    data.extend_from_slice(&RemovePluginV1InstructionArgs { plugin_type }.try_to_vec()?);

    let mut metas = vec![AccountMeta::new(*accounts.asset.key, false)];
    push_optional_meta(&mut metas, accounts.collection, true, false);
    metas.push(AccountMeta::new(*accounts.payer.key, true));
    push_optional_meta(&mut metas, accounts.authority, false, true);
    metas.push(AccountMeta::new_readonly(*accounts.system_program.key, false));
    metas.push(AccountMeta::new_readonly(MPL_CORE_ID, false));

    let ix = Instruction {
        program_id: MPL_CORE_ID,
        accounts: metas,
        data,
    };

    let mut infos = vec![accounts.program.clone(), accounts.asset.clone()];
    push_optional_info(&mut infos, accounts.collection);
    infos.push(accounts.payer.clone());
    push_optional_info(&mut infos, accounts.authority);
    infos.push(accounts.system_program.clone());

    invoke(&ix, &infos).map_err(Into::into)
}

pub struct TransferV1Accounts<'a, 'info> {
    pub program: &'a AccountInfo<'info>,
    pub asset: &'a AccountInfo<'info>,
    pub collection: Option<&'a AccountInfo<'info>>,
    pub payer: &'a AccountInfo<'info>,
    pub authority: Option<&'a AccountInfo<'info>>,
    pub new_owner: &'a AccountInfo<'info>,
}

pub fn transfer_v1<'a, 'info>(
    accounts: TransferV1Accounts<'a, 'info>,
    signers_seeds: &[&[&[u8]]],
) -> Result<()> {
    let mut data = TransferV1InstructionData { discriminator: 14 }.try_to_vec()?;
    data.extend_from_slice(
        &TransferV1InstructionArgs {
            compression_proof: None,
        }
        .try_to_vec()?,
    );

    let mut metas = vec![AccountMeta::new(*accounts.asset.key, false)];
    push_optional_meta(&mut metas, accounts.collection, false, false);
    metas.push(AccountMeta::new(*accounts.payer.key, true));
    push_optional_meta(&mut metas, accounts.authority, false, true);
    metas.push(AccountMeta::new_readonly(*accounts.new_owner.key, false));
    metas.push(AccountMeta::new_readonly(MPL_CORE_ID, false));
    metas.push(AccountMeta::new_readonly(MPL_CORE_ID, false));

    let ix = Instruction {
        program_id: MPL_CORE_ID,
        accounts: metas,
        data,
    };

    let mut infos = vec![accounts.program.clone(), accounts.asset.clone()];
    push_optional_info(&mut infos, accounts.collection);
    infos.push(accounts.payer.clone());
    push_optional_info(&mut infos, accounts.authority);
    infos.push(accounts.new_owner.clone());

    invoke_mpl(ix, &infos, signers_seeds)
}

fn push_optional_meta(
    metas: &mut Vec<AccountMeta>,
    account: Option<&AccountInfo>,
    is_writable: bool,
    is_signer: bool,
) {
    if let Some(account) = account {
        let meta = if is_writable {
            AccountMeta::new(*account.key, is_signer)
        } else {
            AccountMeta::new_readonly(*account.key, is_signer)
        };
        metas.push(meta);
    } else {
        metas.push(AccountMeta::new_readonly(MPL_CORE_ID, false));
    }
}

fn push_optional_info<'info>(
    infos: &mut Vec<AccountInfo<'info>>,
    account: Option<&AccountInfo<'info>>,
) {
    if let Some(account) = account {
        infos.push(account.clone());
    }
}

fn invoke_mpl(
    instruction: Instruction,
    infos: &[AccountInfo],
    signers_seeds: &[&[&[u8]]],
) -> Result<()> {
    if signers_seeds.is_empty() {
        invoke(&instruction, infos).map_err(Into::into)
    } else {
        invoke_signed(&instruction, infos, signers_seeds).map_err(Into::into)
    }
}
