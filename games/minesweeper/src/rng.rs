//! Tiny xorshift32 RNG. Same algorithm as Bullet Hell's `rand01`,
//! duplicated here because the Rule of Three hasn't fired yet (2nd
//! occurrence). Extract to stdlib when a 3rd game needs it.

/// Advance the RNG state and return a value in `[0, 1)`.
pub fn rand01(state: &mut u32) -> f32 {
    let mut x = *state;
    if x == 0 {
        x = 0x1234_5678;
    }
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

/// Return a value in `[0, n)` as a `usize`.
pub fn rand_usize(state: &mut u32, n: usize) -> usize {
    (rand01(state) * n as f32) as usize % n.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rand01_in_range() {
        let mut s = 0xDEAD_BEEF;
        for _ in 0..1000 {
            let v = rand01(&mut s);
            assert!((0.0..1.0).contains(&v), "got {v}");
        }
    }

    #[test]
    fn test_rand01_stable_seed() {
        let mut a = 42u32;
        let mut b = 42u32;
        assert_eq!(rand01(&mut a), rand01(&mut b));
    }

    #[test]
    fn test_rand_usize_bounds() {
        let mut s = 7u32;
        for _ in 0..100 {
            assert!(rand_usize(&mut s, 10) < 10);
            assert!(rand_usize(&mut s, 1) == 0);
        }
    }

    #[test]
    fn test_zero_seed_does_not_stick() {
        let mut s = 0u32;
        let v = rand01(&mut s);
        assert!(s != 0, "state should advance from zero");
        assert!((0.0..1.0).contains(&v));
    }
}