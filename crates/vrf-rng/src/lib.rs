//! VRF-backed RNG for Dungeons & Moles Solana programs.
//!
//! Provides:
//! - [`GameRng`]: Drop-in XorShift 13-7-17 PRNG, seeded from VRF or legacy seeds
//! - [`domains`]: Domain separation constants for independent PRNG streams
//! - [`VrfStatus`]: VRF lifecycle enum shared across programs
//! - [`mock`]: Test helpers for deterministic VRF simulation (feature-gated)

// =============================================================================
// VRF Status & Space
// =============================================================================

/// VRF request lifecycle stage.
///
/// Shared across all programs that manage VRF state accounts.
/// Each program defines its own `#[account] VrfState` struct using this enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "anchor",
    derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize)
)]
pub enum VrfStatus {
    Requested,
    Fulfilled,
    Consumed,
}

/// Account space for VrfState accounts (discriminator + fields).
///
/// Layout: 8 (disc) + 32 (session) + 32 (randomness) + 8 (nonce) + 1 (status) + 1 (bump) = 82
pub const VRF_STATE_SPACE: usize = 8 + 32 + 32 + 8 + 1 + 1;

// =============================================================================
// Domain Separation Constants
// =============================================================================

/// Domain constants for VRF-backed PRNG streams.
///
/// Each domain produces an independent PRNG sequence from the same 32-byte
/// VRF randomness. Domains are XORed with the nonce before seeding.
pub mod domains {
    pub const MAP_GENERATION: u64 = 0x0001;
    pub const POI_SUPPLY_CACHE: u64 = 0x0010;
    pub const POI_TOOL_CRATE: u64 = 0x0011;
    pub const POI_GEODE_VAULT: u64 = 0x0012;
    pub const POI_COUNTER_CACHE: u64 = 0x0013;
    pub const POI_SMUGGLER_HATCH: u64 = 0x0014;
    pub const POI_TOOL_OIL: u64 = 0x0015;
    pub const POI_REROLL: u64 = 0x0016;
    pub const DUEL_BOSS: u64 = 0x0020;
    pub const PIT_DRAFT_INVENTORY: u64 = 0x0030;
    pub const PIT_DRAFT_GOLD: u64 = 0x0031;
    pub const PIT_DRAFT_TIEBREAKER: u64 = 0x0032;
    pub const GAUNTLET_ECHO_DRAW: u64 = 0x0040;
    pub const GAUNTLET_RESERVOIR: u64 = 0x0041;
}

// =============================================================================
// GameRng
// =============================================================================

/// Deterministic PRNG using XorShift 13-7-17.
///
/// Drop-in replacement for `SeededRNG` (map-generator) and `Xorshift64` (poi-system).
/// Can be seeded from VRF randomness with domain separation, or from a legacy seed.
#[derive(Clone, Copy, Debug)]
pub struct GameRng {
    state: u64,
}

impl GameRng {
    /// Create from VRF randomness with domain separation.
    ///
    /// Derives a seed: `u64::from_le_bytes(randomness[0..8]) ^ (nonce ^ domain)`.
    /// Each (nonce, domain) pair produces an independent PRNG stream.
    pub fn from_vrf(randomness: &[u8; 32], nonce: u64, domain: u64) -> Self {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&randomness[..8]);
        let base = u64::from_le_bytes(bytes);
        let seed = base ^ (nonce ^ domain);
        Self::from_seed(seed)
    }

    /// Create from a legacy seed (backward compatible with SeededRNG / Xorshift64).
    pub fn from_seed(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    /// Create from VRF if available, otherwise from legacy seed.
    ///
    /// - `vrf`: `Some((randomness, nonce))` when VRF has been fulfilled
    /// - `domain`: domain separation constant from [`domains`]
    /// - `legacy_seed`: fallback seed when VRF is unavailable
    pub fn new(vrf: Option<(&[u8; 32], u64)>, domain: u64, legacy_seed: u64) -> Self {
        match vrf {
            Some((randomness, nonce)) => Self::from_vrf(randomness, nonce, domain),
            None => Self::from_seed(legacy_seed),
        }
    }

    /// Returns the next random u64 using XorShift 13-7-17.
    #[inline]
    pub fn next_val(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Returns a random integer in the range [min, max] (inclusive).
    pub fn next_int(&mut self, min: u64, max: u64) -> u64 {
        if min >= max {
            return min;
        }
        let range = max.saturating_sub(min).saturating_add(1);
        min.saturating_add(self.next_val() % range)
    }

    /// Returns a random f64 in the range [0.0, 1.0).
    pub fn next_float(&mut self) -> f64 {
        (self.next_val() as f64) / (u64::MAX as f64)
    }

    /// Returns true with the given probability (0.0 to 1.0).
    pub fn next_bool(&mut self, probability: f64) -> bool {
        self.next_float() < probability
    }

    /// Returns a random element from a slice.
    pub fn choose<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            None
        } else {
            let index = self.next_int(0, (items.len() - 1) as u64) as usize;
            items.get(index)
        }
    }

    /// Shuffles a slice in place using Fisher-Yates algorithm.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        let len = items.len();
        if len <= 1 {
            return;
        }
        for i in (1..len).rev() {
            let j = self.next_int(0, i as u64) as usize;
            items.swap(i, j);
        }
    }

    /// Returns a random u64 in [0, max) range.
    pub fn next_bounded(&mut self, max: u64) -> u64 {
        if max == 0 {
            return 0;
        }
        self.next_val() % max
    }

    /// Get current internal state.
    pub fn state(&self) -> u64 {
        self.state
    }
}

// =============================================================================
// Mock VRF Helpers
// =============================================================================

/// Mock VRF helpers for deterministic testing.
///
/// Available when the `mock-vrf` feature is enabled, or during `cargo test`.
#[cfg(any(test, feature = "mock-vrf"))]
pub mod mock {
    /// Generate deterministic 32-byte "randomness" from a seed.
    ///
    /// Uses SplitMix64 to expand the seed into 32 bytes.
    /// NOT cryptographically secure — for testing only.
    pub fn deterministic_randomness(seed: u64) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        let mut state = seed;
        for chunk in bytes.chunks_exact_mut(8) {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^= z >> 31;
            chunk.copy_from_slice(&z.to_le_bytes());
        }
        bytes
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Determinism ----

    #[test]
    fn test_determinism() {
        let mut rng1 = GameRng::from_seed(12345);
        let mut rng2 = GameRng::from_seed(12345);
        for _ in 0..100 {
            assert_eq!(rng1.next_val(), rng2.next_val());
        }
    }

    #[test]
    fn test_different_seeds() {
        let mut rng1 = GameRng::from_seed(12345);
        let mut rng2 = GameRng::from_seed(54321);
        assert_ne!(rng1.next_val(), rng2.next_val());
    }

    #[test]
    fn test_zero_seed_handled() {
        let mut rng = GameRng::from_seed(0);
        assert!(rng.next_val() > 0);
    }

    // ---- Matches existing SeededRNG / TypeScript ----

    #[test]
    fn test_matches_typescript_xorshift() {
        // Same known values as SeededRNG test in map-generator/src/rng.rs
        let mut rng = GameRng::from_seed(1);
        assert_eq!(rng.next_val(), 1082269761);
        assert_eq!(rng.next_val(), 1152992998833853505);
        assert_eq!(rng.next_val(), 11177516664432764457);
    }

    // ---- VRF seeding ----

    #[test]
    fn test_vrf_deterministic() {
        let randomness = [42u8; 32];
        let mut rng1 = GameRng::from_vrf(&randomness, 1, domains::MAP_GENERATION);
        let mut rng2 = GameRng::from_vrf(&randomness, 1, domains::MAP_GENERATION);
        for _ in 0..100 {
            assert_eq!(rng1.next_val(), rng2.next_val());
        }
    }

    // ---- Domain separation ----

    #[test]
    fn test_different_domains() {
        let randomness = [42u8; 32];
        let mut rng1 = GameRng::from_vrf(&randomness, 1, domains::MAP_GENERATION);
        let mut rng2 = GameRng::from_vrf(&randomness, 1, domains::DUEL_BOSS);
        assert_ne!(rng1.next_val(), rng2.next_val());
    }

    #[test]
    fn test_different_nonces() {
        let randomness = [42u8; 32];
        let mut rng1 = GameRng::from_vrf(&randomness, 1, domains::MAP_GENERATION);
        let mut rng2 = GameRng::from_vrf(&randomness, 2, domains::MAP_GENERATION);
        assert_ne!(rng1.next_val(), rng2.next_val());
    }

    #[test]
    fn test_all_domains_unique() {
        let randomness = [42u8; 32];
        let all_domains = [
            domains::MAP_GENERATION,
            domains::POI_SUPPLY_CACHE,
            domains::POI_TOOL_CRATE,
            domains::POI_GEODE_VAULT,
            domains::POI_COUNTER_CACHE,
            domains::POI_SMUGGLER_HATCH,
            domains::POI_TOOL_OIL,
            domains::POI_REROLL,
            domains::DUEL_BOSS,
            domains::PIT_DRAFT_INVENTORY,
            domains::PIT_DRAFT_GOLD,
            domains::GAUNTLET_ECHO_DRAW,
            domains::GAUNTLET_RESERVOIR,
        ];

        let first_vals: Vec<u64> = all_domains
            .iter()
            .map(|&d| GameRng::from_vrf(&randomness, 1, d).next_val())
            .collect();

        for i in 0..first_vals.len() {
            for j in (i + 1)..first_vals.len() {
                assert_ne!(
                    first_vals[i], first_vals[j],
                    "Domains {:#06x} and {:#06x} produced same first value",
                    all_domains[i], all_domains[j]
                );
            }
        }
    }

    // ---- Legacy fallback ----

    #[test]
    fn test_new_none_matches_from_seed() {
        let mut rng1 = GameRng::new(None, domains::MAP_GENERATION, 12345);
        let mut rng2 = GameRng::from_seed(12345);
        for _ in 0..100 {
            assert_eq!(rng1.next_val(), rng2.next_val());
        }
    }

    #[test]
    fn test_new_some_matches_from_vrf() {
        let randomness = [42u8; 32];
        let mut rng1 = GameRng::new(Some((&randomness, 1)), domains::MAP_GENERATION, 12345);
        let mut rng2 = GameRng::from_vrf(&randomness, 1, domains::MAP_GENERATION);
        for _ in 0..100 {
            assert_eq!(rng1.next_val(), rng2.next_val());
        }
    }

    // ---- Range methods ----

    #[test]
    fn test_next_int_range() {
        let mut rng = GameRng::from_seed(42);
        for _ in 0..1000 {
            let val = rng.next_int(10, 20);
            assert!((10..=20).contains(&val));
        }
    }

    #[test]
    fn test_next_int_equal_min_max() {
        let mut rng = GameRng::from_seed(42);
        assert_eq!(rng.next_int(5, 5), 5);
    }

    #[test]
    fn test_next_float_range() {
        let mut rng = GameRng::from_seed(42);
        for _ in 0..1000 {
            let val = rng.next_float();
            assert!((0.0..1.0).contains(&val));
        }
    }

    #[test]
    fn test_next_bounded_range() {
        let mut rng = GameRng::from_seed(12345);
        for _ in 0..1000 {
            assert!(rng.next_bounded(100) < 100);
        }
    }

    #[test]
    fn test_next_bounded_zero() {
        let mut rng = GameRng::from_seed(12345);
        assert_eq!(rng.next_bounded(0), 0);
    }

    // ---- choose / shuffle ----

    #[test]
    fn test_choose_empty() {
        let mut rng = GameRng::from_seed(42);
        let items: Vec<i32> = vec![];
        assert!(rng.choose(&items).is_none());
    }

    #[test]
    fn test_choose_single() {
        let mut rng = GameRng::from_seed(42);
        assert_eq!(rng.choose(&[42]), Some(&42));
    }

    #[test]
    fn test_shuffle_determinism() {
        let mut rng1 = GameRng::from_seed(42);
        let mut rng2 = GameRng::from_seed(42);
        let mut a = vec![1, 2, 3, 4, 5];
        let mut b = vec![1, 2, 3, 4, 5];
        rng1.shuffle(&mut a);
        rng2.shuffle(&mut b);
        assert_eq!(a, b);
    }

    // ---- Mock VRF ----

    #[test]
    fn test_mock_deterministic_randomness() {
        let r1 = mock::deterministic_randomness(42);
        let r2 = mock::deterministic_randomness(42);
        assert_eq!(r1, r2);
        assert_ne!(r1, mock::deterministic_randomness(43));
    }

    #[test]
    fn test_mock_randomness_nonzero() {
        let randomness = mock::deterministic_randomness(42);
        assert_ne!(randomness, [0u8; 32]);
    }

    #[test]
    fn test_mock_to_game_rng() {
        let randomness = mock::deterministic_randomness(42);
        let mut rng = GameRng::from_vrf(&randomness, 1, domains::MAP_GENERATION);
        assert_ne!(rng.next_val(), 0);
    }

    // ---- VrfStatus ----

    #[test]
    fn test_vrf_status_equality() {
        assert_eq!(VrfStatus::Requested, VrfStatus::Requested);
        assert_ne!(VrfStatus::Requested, VrfStatus::Fulfilled);
        assert_ne!(VrfStatus::Fulfilled, VrfStatus::Consumed);
    }
}
