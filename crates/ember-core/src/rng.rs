//! Tiny xorshift32 RNG.
//!
//! Not cryptographic. Deterministic given a seed. Used by Bullet Hell,
//! Minesweeper, and Simon (previously duplicated 3 times — Rule of Three
//! fired, extracted here).
//!
//! ## Usage
//!
//! ```
//! use ember_core::rng::Rng;
//!
//! let mut rng = Rng::new(42);
//! let a = rng.next_u32();
//! let b = rng.next_f32();
//! let c = rng.next_range(10);
//! # let _ = (a, b, c);
//! ```
//!
//! ## Migration from `&mut u32`
//!
//! Older code stores the RNG state as a bare `u32` and passes `&mut u32`
//! around. Two free functions preserve that style while delegating to
//! [`Rng`]: [`rand01`] and [`rand_usize`].

/// An xorshift32 RNG.
///
/// The state must never be zero (xorshift would get stuck). `new(0)` and
/// `set_state(0)` substitute `0xDEAD_BEEF` instead.
#[derive(Debug, Clone)]
pub struct Rng {
    state: u32,
}

impl Rng {
    /// Fallback state used when the caller passes `0` as a seed.
    pub const ZERO_SEED_FALLBACK: u32 = 0xDEAD_BEEF;

    /// Create an RNG from a seed. Seed `0` is replaced by
    /// [`Self::ZERO_SEED_FALLBACK`] to avoid the degenerate all-zero state.
    pub fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 {
                Self::ZERO_SEED_FALLBACK
            } else {
                seed
            },
        }
    }

    /// Build an RNG from a raw state value.
    ///
    /// Equivalent to [`Self::new`]; provided for symmetry with
    /// [`Self::state`] and to make migration from `&mut u32` explicit.
    pub fn from_state(state: u32) -> Self {
        Self::new(state)
    }

    /// Current raw state. Useful for persisting, snapshotting, or bridging
    /// to `&mut u32`-style APIs.
    pub fn state(&self) -> u32 {
        self.state
    }

    /// Replace the raw state. `0` is substituted by
    /// [`Self::ZERO_SEED_FALLBACK`].
    pub fn set_state(&mut self, state: u32) {
        self.state = if state == 0 {
            Self::ZERO_SEED_FALLBACK
        } else {
            state
        };
    }

    /// Next raw `u32`. Advances the state.
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// Uniform `f32` in `[0, 1)`.
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform integer in `[0, n)`. Returns `0` if `n == 0`.
    pub fn next_range(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next_u32() as usize) % n
    }

    /// Uniform `f32` in `[min, max)`.
    pub fn next_f32_range(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }

    /// Uniform `bool` (50/50).
    pub fn next_bool(&mut self) -> bool {
        self.next_u32() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// Free helpers — bridge to `&mut u32`-style APIs
// ---------------------------------------------------------------------------

/// Advance the state and return a uniform `f32` in `[0, 1)`.
///
/// Equivalent to `Rng::from_state(*state).next_f32()`, but writes the
/// advanced state back into `*state`.
pub fn rand01(state: &mut u32) -> f32 {
    let mut rng = Rng::from_state(*state);
    let v = rng.next_f32();
    *state = rng.state();
    v
}

/// Return a uniform `usize` in `[0, n)`. Returns `0` if `n == 0`.
///
/// Equivalent to `Rng::from_state(*state).next_range(n)`, but writes the
/// advanced state back into `*state`.
pub fn rand_usize(state: &mut u32, n: usize) -> usize {
    let mut rng = Rng::from_state(*state);
    let v = rng.next_range(n);
    *state = rng.state();
    v
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
        assert_ne!(v1, 0);
    }

    #[test]
    fn test_set_state_zero_uses_fallback() {
        let mut r = Rng::new(42);
        r.set_state(0);
        assert_eq!(r.state(), Rng::ZERO_SEED_FALLBACK);
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

    #[test]
    fn test_next_f32_range() {
        let mut r = Rng::new(99);
        for _ in 0..1000 {
            let v = r.next_f32_range(-5.0, 5.0);
            assert!((-5.0..5.0).contains(&v), "got {v}");
        }
    }

    #[test]
    fn test_next_bool_is_balanced() {
        let mut r = Rng::new(0xDEAD_BEEF);
        let mut trues = 0;
        for _ in 0..10_000 {
            if r.next_bool() {
                trues += 1;
            }
        }
        // 10 000 tirages, ~50% de true. Tolérance large (45–55%).
        assert!((4500..5500).contains(&trues), "got {trues}");
    }

    #[test]
    fn test_state_roundtrip() {
        let mut r = Rng::new(42);
        r.next_u32();
        r.next_u32();
        let s = r.state();
        let mut r2 = Rng::from_state(s);
        assert_eq!(r.next_u32(), r2.next_u32());
    }

    #[test]
    fn test_rand01_helper_matches_rng() {
        let mut state = 42u32;
        let v1 = rand01(&mut state);
        let mut rng = Rng::from_state(42);
        let v2 = rng.next_f32();
        assert_eq!(v1, v2);
        assert_eq!(state, rng.state());
    }

    #[test]
    fn test_rand_usize_helper_matches_rng() {
        let mut state = 7u32;
        let v1 = rand_usize(&mut state, 10);
        let mut rng = Rng::from_state(7);
        let v2 = rng.next_range(10);
        assert_eq!(v1, v2);
        assert_eq!(state, rng.state());
    }
}
