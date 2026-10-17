//! Deterministic RNG. Hand-rolled so generated worlds are byte-identical
//! for a given `(seed, kind)` across runs and platforms, with no `rand`
//! dependency (works on wasm without extra plumbing).
//!
//! **Xorshift64\*** — small, fast, good enough for scatter.

#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u32) -> Self {
        // Golden-ratio multiplier spreads close seeds apart.
        let s = (seed as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        Self { state: s.max(1) } // xorshift locks up at state = 0
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }

    /// Uniform float in `[0, 1)`.
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform int in `[lo, hi)`.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(hi > lo);
        lo + (self.next_u32() % (hi - lo) as u32) as i32
    }

    pub fn chance(&mut self, p: f32) -> bool {
        self.next_f32() < p.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = Rng::new(1);
        let mut b = Rng::new(2);
        let mut diffs = 0;
        for _ in 0..100 {
            if a.next_u32() != b.next_u32() { diffs += 1; }
        }
        assert!(diffs > 90, "only {diffs}/100 differences");
    }

    #[test]
    fn seed_zero_does_not_lock_up() {
        let mut r = Rng::new(0);
        let first = r.next_u32();
        let second = r.next_u32();
        assert_ne!(first, 0);
        assert_ne!(first, second);
    }

    #[test]
    fn range_stays_in_bounds() {
        let mut r = Rng::new(7);
        for _ in 0..1000 {
            let v = r.range(-3, 5);
            assert!((-3..5).contains(&v), "got {v}");
        }
    }

    #[test]
    fn next_f32_is_unit_interval() {
        let mut r = Rng::new(99);
        for _ in 0..1000 {
            let v = r.next_f32();
            assert!((0.0..1.0).contains(&v), "got {v}");
        }
    }

    #[test]
    fn chance_zero_and_one_are_degenerate() {
        let mut r = Rng::new(3);
        for _ in 0..50 {
            assert!(!r.chance(0.0));
            assert!(r.chance(1.0));
        }
    }
}