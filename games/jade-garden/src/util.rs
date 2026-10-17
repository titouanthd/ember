//! Small shared utilities.
//!
//! Both functions were previously duplicated in this crate:
//! - `fmt_time` was copied verbatim in `hud.rs`, `menu.rs`, `main.rs`.
//! - `hash_u32` was copied verbatim in `scroll_painting.rs` and
//!   `tile_render.rs` (and the same function exists in `ember-wars`).
//!
//! Consolidated here for now. `hash_u32` is a candidate for
//! `ember_core::rng::hash_u32` once a third game uses it.

/// Formats a duration in seconds as `M:SS` (no leading zero on the
/// minutes). Rounds to the nearest second.
///
/// `fmt_time(0.0) == "0:00"`, `fmt_time(65.4) == "1:05"`.
pub fn fmt_time(secs: f32) -> String {
    let total = secs.max(0.0).round() as u32;
    format!("{}:{:02}", total / 60, total % 60)
}

/// Deterministic 32-bit hash (murmur-inspired).
///
/// Same `(seed, x)` → same output, across runs and platforms. Used
/// to derive stable procedural details from a seed (mountain ridges
/// in `scroll_painting`, jade veins in `tile_render`).
///
/// Not cryptographic. Do not use for anything security-adjacent.
pub fn hash_u32(seed: u32, x: u32) -> u32 {
    let mut h = seed ^ x.wrapping_mul(0x9E37_79B9);
    h ^= h >> 16;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 16;
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- fmt_time ----------

    #[test]
    fn fmt_time_zero() {
        assert_eq!(fmt_time(0.0), "0:00");
    }

    #[test]
    fn fmt_time_below_a_minute() {
        assert_eq!(fmt_time(5.0), "0:05");
        assert_eq!(fmt_time(59.4), "0:59");
    }

    #[test]
    fn fmt_time_rounds_up_to_the_next_minute() {
        assert_eq!(fmt_time(59.6), "1:00");
    }

    #[test]
    fn fmt_time_minutes_and_hours() {
        assert_eq!(fmt_time(60.0), "1:00");
        assert_eq!(fmt_time(65.0), "1:05");
        assert_eq!(fmt_time(3599.0), "59:59");
        assert_eq!(fmt_time(3600.0), "60:00");
    }

    #[test]
    fn fmt_time_rounds_to_nearest() {
        assert_eq!(fmt_time(65.4), "1:05");
        assert_eq!(fmt_time(65.5), "1:06");
    }

    #[test]
    fn fmt_time_negative_clamps_to_zero() {
        assert_eq!(fmt_time(-5.0), "0:00");
    }

    // ---------- hash_u32 ----------

    #[test]
    fn hash_is_deterministic() {
        assert_eq!(hash_u32(42, 100), hash_u32(42, 100));
        assert_eq!(hash_u32(0, 0), hash_u32(0, 0));
        assert_eq!(hash_u32(0xDEAD_BEEF, 5), hash_u32(0xDEAD_BEEF, 5));
    }

    #[test]
    fn hash_changes_with_seed() {
        assert_ne!(hash_u32(1, 100), hash_u32(2, 100));
    }

    #[test]
    fn hash_changes_with_input() {
        assert_ne!(hash_u32(1, 100), hash_u32(1, 101));
    }

    #[test]
    fn hash_seed_and_input_are_not_commutative() {
        // Regression guard: `seed ^ x * K` is not symmetric.
        assert_ne!(hash_u32(1, 2), hash_u32(2, 1));
    }
}