//! Tiny xorshift32 RNG. Same algorithm as Bullet Hell and Minesweeper.
//! Duplicated for now — Session D extracts this into ember_core once we
//! refactor all games together.

/// An xorshift32 RNG. Not cryptographic. Deterministic given a seed.
#[derive(Debug, Clone)]
pub struct Rng {
    state: u32,
}

impl Rng {
    pub fn new(seed: u32) -> Self {
        // Avoid the degenerate all-zero state.
        Self {
            state: if seed == 0 { 0xDEAD_BEEF } else { seed },
        }
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// Uniform in [0, 1).
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform in [0, n). Returns 0 if n == 0.
    pub fn next_range(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next_u32() as usize) % n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_given_seed() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn test_zero_seed_does_not_stick() {
        let mut r = Rng::new(0);
        let v1 = r.next_u32();
        let v2 = r.next_u32();
        assert_ne!(v1, v2);
    }

    #[test]
    fn test_next_f32_in_range() {
        let mut r = Rng::new(7);
        for _ in 0..1000 {
            let v = r.next_f32();
            assert!((0.0..1.0).contains(&v), "got {v}");
        }
    }

    #[test]
    fn test_next_range_bounds() {
        let mut r = Rng::new(3);
        for _ in 0..1000 {
            assert!(r.next_range(4) < 4);
            assert_eq!(r.next_range(0), 0);
            assert_eq!(r.next_range(1), 0);
        }
    }
}