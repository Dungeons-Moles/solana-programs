use quasar_lang::prelude::*;

pub const VRF_PROBE_DISCRIMINATOR: [u8; 8] = [219, 84, 237, 45, 14, 99, 103, 11];

#[account(
    discriminator = [219, 84, 237, 45, 14, 99, 103, 11],
    fixed_capacity,
    set_inner
)]
#[seeds(b"vrf_probe", owner: Address)]
pub struct VrfProbe {
    pub owner: Address,
    pub request_count: u64,
    pub fulfilled_count: u64,
    pub last_request_seed: [u8; 32],
    pub last_randomness: [u8; 32],
    pub last_die_roll: u8,
    pub callback_verified: bool,
    pub bump: u8,
}

impl VrfProbe {
    pub const SEED_PREFIX: &'static [u8] = b"vrf_probe";
}
