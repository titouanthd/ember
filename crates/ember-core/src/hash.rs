//! Deterministic 32-bit hash functions.
//!
//! Used to derive stable procedural details from a seed (mountain
//! ridges, jade veins, star positions). Not cryptographic — do not
//! use for anything security-adjacent.
//!
//! Extracted after 2 games (ember-wars, jade-garden) accumulated
//! 3+ verbatim copies. Consumed by `ember-wars::textures` and
//! `jade-garden::{scroll_painting, tile_render}`.

/// Deterministic 32-bit hash (MurmurHash3 finalizer).
///
/// Same `(seed, x)` → same output, across runs and platforms.
/// Avalanche-better-than-linear: a single-bit change in `x` flips
/// roughly half the output bits.
pub fn hash_u32(seed: u32, x: u32) -> u32 {
    let mut h = seed ^ x.wrapping_mul(0x9E37_79B9);
    h ^= h >> 16;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 16;
    h
}

/// Hash déterministe 2D. Même `(seed, x, y)` → même sortie.
///
/// Chaînage simple : `h(h(seed, x), y)`. Ce n'est pas un vrai hash 2D
/// (pas de diffusion croisée entre x et y), mais c'est stable, rapide,
/// et suffisant pour du procédural visuel.
pub fn hash_u32_2(seed: u32, x: u32, y: u32) -> u32 {
    hash_u32(hash_u32(seed, x), y)
}

/// Hash déterministe 3D. Même `(seed, x, y, z)` → même sortie.
pub fn hash_u32_3(seed: u32, x: u32, y: u32, z: u32) -> u32 {
    hash_u32(hash_u32_2(seed, x, y), z)
}


#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn hash_zero_seed_zero_input_is_valid() {
        // Must not panic. Value is implementation-defined but stable.
        let _ = hash_u32(0, 0);
    }

    #[test]
    fn hash_avalanche_distributes() {
        // A single-bit change in input should flip ~half the output bits.
        let base = hash_u32(0xCAFE, 12345);
        let mut diff_bits = 0u32;
        for shift in 0..32 {
            let flipped = hash_u32(0xCAFE, 12345 ^ (1 << shift));
            diff_bits += (base ^ flipped).count_ones();
        }
        // Expected ≈ 512 flipped bits. Wide margin 300..=724.
        assert!((300..=724).contains(&diff_bits), "avalanche: {diff_bits}");
    }

    #[test]
    fn hash_2d_deterministic_and_distinct() {
        assert_eq!(hash_u32_2(1, 2, 3), hash_u32_2(1, 2, 3));
        assert_ne!(hash_u32_2(1, 2, 3), hash_u32_2(1, 3, 2));
        assert_ne!(hash_u32_2(1, 2, 3), hash_u32_2(2, 2, 3));
    }

    #[test]
    fn hash_3d_deterministic_and_distinct() {
        assert_eq!(hash_u32_3(1, 2, 3, 4), hash_u32_3(1, 2, 3, 4));
        assert_ne!(hash_u32_3(1, 2, 3, 4), hash_u32_3(1, 2, 4, 3));
    }
}