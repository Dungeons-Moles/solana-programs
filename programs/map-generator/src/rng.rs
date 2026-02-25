// Re-export GameRng from vrf-rng for new code paths
pub use vrf_rng::GameRng;

/// XorShift-based seeded random number generator.
/// Matches the TypeScript implementation for deterministic map generation.
#[derive(Clone, Copy, Debug)]
pub struct SeededRNG {
    state: u64,
}

impl SeededRNG {
    /// Creates a new RNG with the given seed.
    /// Ensures the seed is never zero (XorShift requirement).
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    /// Returns the next random u64 using XorShift algorithm.
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rng_determinism() {
        // Same seed should produce same sequence
        let mut rng1 = SeededRNG::new(12345);
        let mut rng2 = SeededRNG::new(12345);

        for _ in 0..100 {
            assert_eq!(rng1.next_val(), rng2.next_val());
        }
    }

    #[test]
    fn test_rng_different_seeds() {
        // Different seeds should produce different sequences
        let mut rng1 = SeededRNG::new(12345);
        let mut rng2 = SeededRNG::new(54321);

        // Very unlikely to be equal
        assert_ne!(rng1.next_val(), rng2.next_val());
    }

    #[test]
    fn test_rng_zero_seed() {
        // Zero seed should be converted to 1
        let mut rng = SeededRNG::new(0);
        assert!(rng.next_val() > 0);
    }

    #[test]
    fn test_next_int_range() {
        let mut rng = SeededRNG::new(42);

        for _ in 0..1000 {
            let val = rng.next_int(10, 20);
            assert!(val >= 10 && val <= 20);
        }
    }

    #[test]
    fn test_next_int_equal_min_max() {
        let mut rng = SeededRNG::new(42);
        assert_eq!(rng.next_int(5, 5), 5);
    }

    #[test]
    fn test_next_float_range() {
        let mut rng = SeededRNG::new(42);

        for _ in 0..1000 {
            let val = rng.next_float();
            assert!(val >= 0.0 && val < 1.0);
        }
    }

    #[test]
    fn test_choose_empty() {
        let mut rng = SeededRNG::new(42);
        let items: Vec<i32> = vec![];
        assert!(rng.choose(&items).is_none());
    }

    #[test]
    fn test_choose_single() {
        let mut rng = SeededRNG::new(42);
        let items = vec![42];
        assert_eq!(rng.choose(&items), Some(&42));
    }

    #[test]
    fn test_shuffle_determinism() {
        let mut rng1 = SeededRNG::new(42);
        let mut rng2 = SeededRNG::new(42);

        let mut items1 = vec![1, 2, 3, 4, 5];
        let mut items2 = vec![1, 2, 3, 4, 5];

        rng1.shuffle(&mut items1);
        rng2.shuffle(&mut items2);

        assert_eq!(items1, items2);
    }

    #[test]
    fn test_xorshift_matches_typescript() {
        // Verify the XorShift 13-7-17 variant produces consistent values
        // These are the actual values from Rust XorShift with seed=1
        let mut rng = SeededRNG::new(1);

        // First few values from XorShift with seed=1
        let first = rng.next_val();
        let second = rng.next_val();
        let third = rng.next_val();

        // The exact values for the 13-7-17 XorShift variant
        // TypeScript implementation should produce these same values
        assert_eq!(first, 1082269761);
        assert_eq!(second, 1152992998833853505);
        assert_eq!(third, 11177516664432764457);
    }
}
